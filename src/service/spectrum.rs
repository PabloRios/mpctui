//! Análisis del espectro, en funciones puras: PCM → mono → ventana de Hann → FFT →
//! bandas logarítmicas → decibeles normalizados → suavizado con picos.

use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use crate::domain::{PcmFormat, SpectrumFrame, band_centers, band_edges};

/// Muestras por análisis: unos 46 ms a 44,1 kHz, con unos 21,5 Hz por bin.
pub const FFT_SIZE: usize = 2048;
/// Nivel que se dibuja vacío y nivel que llena la barra.
pub const FLOOR_DB: f32 = -60.0;
pub const CEIL_DB: f32 = 0.0;
/// Evita el logaritmo de cero.
const MAGNITUDE_EPSILON: f32 = 1e-9;
const DECIBELS_PER_LOG: f32 = 20.0;
/// Cuánto baja una barra por segundo, en alturas completas.
pub const FALL_PER_SECOND: f32 = 1.5;
/// Cuánto se queda quieto un pico antes de empezar a caer.
pub const PEAK_HOLD: Duration = Duration::from_millis(500);
pub const PEAK_FALL_PER_SECOND: f32 = 0.5;
/// Valor máximo de una muestra de 16 bits, para normalizar a [-1, 1].
const SAMPLE_SCALE: f32 = 32_768.0;

/// Convierte bytes de PCM en muestras mono. Guarda los bytes de un cuadro cortado por
/// la mitad hasta la lectura siguiente.
#[derive(Debug)]
pub struct PcmDecoder {
    format: PcmFormat,
    pending: Vec<u8>,
}

impl PcmDecoder {
    pub fn new(format: PcmFormat) -> Self {
        Self {
            format,
            pending: Vec::new(),
        }
    }

    /// Agrega a `samples` el promedio de los canales de cada cuadro completo.
    pub fn decode(&mut self, bytes: &[u8], samples: &mut SampleWindow) {
        self.pending.extend_from_slice(bytes);
        let frame_bytes = self.format.bytes_per_frame();
        let complete = self.pending.len() / frame_bytes * frame_bytes;
        for frame in self.pending[..complete].chunks_exact(frame_bytes) {
            samples.push(self.mono(frame));
        }
        self.pending.drain(..complete);
    }

    /// Promedio de los canales, normalizado a [-1, 1]. El orden de bytes es el nativo.
    fn mono(&self, frame: &[u8]) -> f32 {
        let sum: f32 = frame
            .chunks_exact(self.format.bytes_per_sample())
            .map(|sample| f32::from(i16::from_ne_bytes([sample[0], sample[1]])))
            .sum();
        sum / f32::from(self.format.channels()) / SAMPLE_SCALE
    }
}

/// Las últimas `capacity` muestras; las viejas se descartan.
#[derive(Debug)]
pub struct SampleWindow {
    samples: VecDeque<f32>,
    capacity: usize,
}

impl SampleWindow {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, sample: f32) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Exactamente `capacity` muestras, con ceros adelante si todavía faltan.
    pub fn latest(&self) -> Vec<f32> {
        let missing = self.capacity - self.samples.len();
        let mut result = vec![0.0; missing];
        result.extend(&self.samples);
        result
    }
}

/// Ventana de Hann de `size` puntos.
pub fn hann_window(size: usize) -> Vec<f32> {
    let span = (size - 1) as f32;
    (0..size)
        .map(|index| 0.5 - 0.5 * (2.0 * PI * index as f32 / span).cos())
        .collect()
}

/// Del bloque de muestras a niveles por banda (0..1). Planifica la FFT una sola vez.
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    window_sum: f32,
    /// Hz que cubre cada bin.
    bin_width: f32,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32) -> Self {
        let window = hann_window(FFT_SIZE);
        Self {
            fft: FftPlanner::new().plan_fft_forward(FFT_SIZE),
            window_sum: window.iter().sum(),
            window,
            bin_width: sample_rate as f32 / FFT_SIZE as f32,
        }
    }

    pub fn levels(&self, samples: &[f32], bands: usize) -> Vec<f32> {
        let magnitudes = self.magnitudes(samples);
        band_magnitudes(&magnitudes, self.bin_width, bands)
            .into_iter()
            .map(level)
            .collect()
    }

    /// `|X| × 2 / suma_de_la_ventana` de cada bin hasta Nyquist.
    fn magnitudes(&self, samples: &[f32]) -> Vec<f32> {
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .zip(&self.window)
            .map(|(sample, weight)| Complex::new(sample * weight, 0.0))
            .collect();
        self.fft.process(&mut buffer);
        buffer[..FFT_SIZE / 2]
            .iter()
            .map(|bin| bin.norm() * 2.0 / self.window_sum)
            .collect()
    }
}

/// Máximo de los bins de cada banda. Una banda grave sin bins toma el más cercano a
/// su centro.
pub fn band_magnitudes(magnitudes: &[f32], bin_width: f32, bands: usize) -> Vec<f32> {
    let edges = band_edges(bands);
    let centers = band_centers(bands);
    let last_bin = magnitudes.len().saturating_sub(1);
    edges
        .windows(2)
        .zip(centers)
        .map(|(edge, center)| {
            let first = (edge[0] / bin_width).ceil() as usize;
            let end = ((edge[1] / bin_width).ceil() as usize).min(magnitudes.len());
            if first < end {
                magnitudes[first..end].iter().copied().fold(0.0, f32::max)
            } else {
                let nearest = ((center / bin_width).round() as usize).min(last_bin);
                magnitudes.get(nearest).copied().unwrap_or_default()
            }
        })
        .collect()
}

/// Magnitud a nivel: decibeles normalizados entre `FLOOR_DB` y `CEIL_DB`.
pub fn level(magnitude: f32) -> f32 {
    let decibels = DECIBELS_PER_LOG * (magnitude + MAGNITUDE_EPSILON).log10();
    ((decibels - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0)
}

/// Suaviza la caída de las barras y sostiene los picos.
#[derive(Debug, Default)]
pub struct Smoother {
    levels: Vec<f32>,
    peaks: Vec<f32>,
    /// Desde cuándo está quieto cada pico.
    peak_ages: Vec<Duration>,
}

impl Smoother {
    /// Sube enseguida; baja como mucho `FALL_PER_SECOND × elapsed`. Si cambió la
    /// cantidad de bandas, empieza de cero.
    pub fn update(&mut self, targets: &[f32], elapsed: Duration) {
        if self.levels.len() != targets.len() {
            *self = Self {
                levels: vec![0.0; targets.len()],
                peaks: vec![0.0; targets.len()],
                peak_ages: vec![Duration::ZERO; targets.len()],
            };
        }
        let seconds = elapsed.as_secs_f32();
        for (band, target) in targets.iter().enumerate() {
            let level = &mut self.levels[band];
            *level = target.max(*level - FALL_PER_SECOND * seconds).max(0.0);
            self.update_peak(band, elapsed);
        }
    }

    /// El pico sube con la barra; si no, espera `PEAK_HOLD` y después cae, sin bajar
    /// de la barra.
    fn update_peak(&mut self, band: usize, elapsed: Duration) {
        let level = self.levels[band];
        let peak = &mut self.peaks[band];
        let age = &mut self.peak_ages[band];
        if level >= *peak {
            *peak = level;
            *age = Duration::ZERO;
        } else if *age < PEAK_HOLD {
            *age += elapsed;
        } else {
            *peak = (*peak - PEAK_FALL_PER_SECOND * elapsed.as_secs_f32()).max(level);
        }
    }

    pub fn frame(&self, signal: bool) -> SpectrumFrame {
        SpectrumFrame {
            levels: self.levels.clone(),
            peaks: self.peaks.clone(),
            signal,
        }
    }
}

/// Seno de `frequency` Hz en PCM estéreo de 16 bits a 44,1 kHz, `amplitude` entre 0 y 1.
#[cfg(test)]
pub(crate) fn sine_pcm(frequency: f32, amplitude: f32, frames: usize) -> Vec<u8> {
    const SAMPLE_RATE: f32 = 44_100.0;
    (0..frames)
        .flat_map(|index| {
            let phase = 2.0 * PI * frequency * index as f32 / SAMPLE_RATE;
            let sample = (phase.sin() * amplitude * (SAMPLE_SCALE - 1.0)) as i16;
            let bytes = sample.to_ne_bytes();
            [bytes, bytes].concat()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 44_100;

    fn stereo() -> PcmFormat {
        PcmFormat::parse("44100:16:2").unwrap()
    }

    fn analyze(pcm: &[u8], bands: usize) -> Vec<f32> {
        let mut decoder = PcmDecoder::new(stereo());
        let mut window = SampleWindow::new(FFT_SIZE);
        decoder.decode(pcm, &mut window);
        SpectrumAnalyzer::new(SAMPLE_RATE).levels(&window.latest(), bands)
    }

    #[test]
    fn decoder_keeps_frames_split_between_reads() {
        let left = 16_384_i16.to_ne_bytes();
        let right = (-16_384_i16).to_ne_bytes();
        let louder = 32_767_i16.to_ne_bytes();
        let bytes = [left, right, louder, louder].concat();
        let mut decoder = PcmDecoder::new(stereo());
        let mut window = SampleWindow::new(2);
        decoder.decode(&bytes[..3], &mut window);
        assert_eq!(window.latest(), [0.0, 0.0]);
        decoder.decode(&bytes[3..6], &mut window);
        assert_eq!(window.latest(), [0.0, 0.0]);
        decoder.decode(&bytes[6..], &mut window);
        let latest = window.latest();
        assert_eq!(latest[0], 0.0);
        assert!((latest[1] - 1.0).abs() < 0.001);
    }

    #[test]
    fn sample_window_keeps_only_the_latest() {
        let mut window = SampleWindow::new(3);
        for sample in [1.0, 2.0, 3.0, 4.0] {
            window.push(sample);
        }
        assert_eq!(window.latest(), [2.0, 3.0, 4.0]);
    }

    #[test]
    fn hann_window_is_zero_at_the_ends_and_one_in_the_middle() {
        let window = hann_window(5);
        assert!(window[0].abs() < 1e-6 && window[4].abs() < 1e-6);
        assert!((window[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_1_khz_sine_peaks_in_the_band_that_contains_it() {
        let bands = 32;
        let levels = analyze(&sine_pcm(1_000.0, 0.5, FFT_SIZE), bands);
        let loudest = levels
            .iter()
            .enumerate()
            .max_by(|first, second| first.1.total_cmp(second.1))
            .map(|(band, _)| band)
            .unwrap();
        let edges = band_edges(bands);
        assert!(edges[loudest] <= 1_000.0 && 1_000.0 < edges[loudest + 1]);
        assert!(levels[loudest] > 0.8, "{levels:?}");
    }

    #[test]
    fn silence_has_every_band_at_zero() {
        let levels = analyze(&vec![0; FFT_SIZE * 4], 16);
        assert!(levels.iter().all(|level| *level == 0.0), "{levels:?}");
    }

    #[test]
    fn low_bands_without_bins_take_the_nearest_one() {
        // Bins de 1 kHz: las bandas graves no contienen ninguno.
        let magnitudes = [0.0, 1.0, 0.5, 0.25];
        let bands = band_magnitudes(&magnitudes, 1_000.0, 8);
        assert_eq!(bands[0], 0.0);
        assert!(bands.iter().all(|band| *band <= 1.0));
        assert_eq!(*bands.last().unwrap(), 0.25);
    }

    #[test]
    fn levels_are_normalized_decibels() {
        assert_eq!(level(1.0), 1.0);
        assert_eq!(level(0.0), 0.0);
        assert!((level(0.031_6) - 0.5).abs() < 0.01);
    }

    #[test]
    fn bars_rise_at_once_and_fall_slowly() {
        let mut smoother = Smoother::default();
        let tick = Duration::from_millis(100);
        smoother.update(&[1.0], tick);
        assert_eq!(smoother.frame(true).levels, [1.0]);
        smoother.update(&[0.0], tick);
        let fallen = 1.0 - FALL_PER_SECOND * tick.as_secs_f32();
        assert!((smoother.frame(true).levels[0] - fallen).abs() < 1e-6);
        smoother.update(&[0.0], Duration::from_secs(5));
        assert_eq!(smoother.frame(false).levels, [0.0]);
    }

    #[test]
    fn peaks_hold_and_then_fall() {
        let mut smoother = Smoother::default();
        smoother.update(&[1.0], Duration::ZERO);
        smoother.update(&[0.0], Duration::from_millis(400));
        assert_eq!(smoother.frame(true).peaks, [1.0]);
        smoother.update(&[0.0], Duration::from_millis(200));
        assert_eq!(smoother.frame(true).peaks, [1.0]);
        smoother.update(&[0.0], Duration::from_millis(200));
        let peak = smoother.frame(true).peaks[0];
        assert!(peak < 1.0 && peak > 0.8, "{peak}");
        smoother.update(&[0.0, 0.0], Duration::ZERO);
        assert_eq!(smoother.frame(true).peaks, [0.0, 0.0]);
    }
}
