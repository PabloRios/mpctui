//! Analizador de bandas, en funciones puras: muestras → ventana de Hann → FFT → bandas
//! logarítmicas → decibeles normalizados → suavizado con picos. Lo usan Barras, Espejo
//! (una vez por canal) y Cascada.

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use super::levels::decibels;
use super::peak::{HeldPeak, PeakDecay};
use crate::domain::{BandFrame, band_centers, band_edges};

/// Muestras por análisis: unos 46 ms a 44,1 kHz, con unos 21,5 Hz por bin.
pub const FFT_SIZE: usize = 2048;
/// Nivel que se dibuja vacío y nivel que llena la barra.
pub const FLOOR_DB: f32 = -60.0;
pub const CEIL_DB: f32 = 0.0;
/// Cuánto baja una barra por segundo, en alturas completas.
pub const FALL_PER_SECOND: f32 = 1.5;
/// Pico de las barras: medio segundo quieto y después cae.
pub const BAND_PEAK_DECAY: PeakDecay = PeakDecay {
    hold: Duration::from_millis(500),
    fall_per_second: 0.5,
};

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

    /// `samples` son las últimas `FFT_SIZE` muestras.
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
    ((decibels(magnitude) - FLOOR_DB) / (CEIL_DB - FLOOR_DB)).clamp(0.0, 1.0)
}

/// Suaviza la caída de las barras y sostiene los picos.
#[derive(Debug, Default)]
pub struct Smoother {
    levels: Vec<f32>,
    peaks: Vec<HeldPeak>,
}

impl Smoother {
    /// Sube enseguida; baja como mucho `FALL_PER_SECOND × elapsed`. Si cambió la
    /// cantidad de bandas, empieza de cero.
    pub fn update(&mut self, targets: &[f32], elapsed: Duration) {
        if self.levels.len() != targets.len() {
            *self = Self {
                levels: vec![0.0; targets.len()],
                peaks: vec![HeldPeak::new(0.0); targets.len()],
            };
        }
        let seconds = elapsed.as_secs_f32();
        for ((level, peak), target) in self.levels.iter_mut().zip(&mut self.peaks).zip(targets) {
            *level = target.max(*level - FALL_PER_SECOND * seconds).max(0.0);
            peak.update(*level, elapsed, BAND_PEAK_DECAY);
        }
    }

    pub fn frame(&self) -> BandFrame {
        BandFrame {
            levels: self.levels.clone(),
            peaks: self.peaks.iter().map(|peak| peak.value()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::visual::pcm::sine_samples;

    const SAMPLE_RATE: u32 = 44_100;

    fn analyze(samples: &[f32], bands: usize) -> Vec<f32> {
        SpectrumAnalyzer::new(SAMPLE_RATE).levels(samples, bands)
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
        let levels = analyze(&sine_samples(1_000.0, 0.5, FFT_SIZE), bands);
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
        let levels = analyze(&[0.0; FFT_SIZE], 16);
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
        assert_eq!(smoother.frame().levels, [1.0]);
        smoother.update(&[0.0], tick);
        let fallen = 1.0 - FALL_PER_SECOND * tick.as_secs_f32();
        assert!((smoother.frame().levels[0] - fallen).abs() < 1e-6);
        smoother.update(&[0.0], Duration::from_secs(5));
        assert_eq!(smoother.frame().levels, [0.0]);
    }

    #[test]
    fn peaks_hold_and_then_fall() {
        let mut smoother = Smoother::default();
        smoother.update(&[1.0], Duration::ZERO);
        smoother.update(&[0.0], Duration::from_millis(400));
        assert_eq!(smoother.frame().peaks, [1.0]);
        smoother.update(&[0.0], Duration::from_millis(200));
        assert_eq!(smoother.frame().peaks, [1.0]);
        smoother.update(&[0.0], Duration::from_millis(200));
        let peak = smoother.frame().peaks[0];
        assert!(peak < 1.0 && peak > 0.8, "{peak}");
        smoother.update(&[0.0, 0.0], Duration::ZERO);
        assert_eq!(smoother.frame().peaks, [0.0, 0.0]);
    }
}
