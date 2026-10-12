//! Del PCM crudo de la salida fifo a pares de muestras izquierda/derecha.

use std::collections::VecDeque;

use crate::domain::PcmFormat;

/// Valor máximo de una muestra de 16 bits, para normalizar a [-1, 1].
pub const SAMPLE_SCALE: f32 = 32_768.0;

/// Una muestra por canal, normalizada a [-1, 1]. Con un solo canal, los dos son iguales.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SamplePair {
    pub left: f32,
    pub right: f32,
}

impl SamplePair {
    /// Promedio de los canales: lo que escuchan las formas mono.
    pub fn mono(self) -> f32 {
        (self.left + self.right) / 2.0
    }
}

/// Convierte bytes de PCM en pares de muestras. Guarda los bytes de un cuadro cortado
/// por la mitad hasta la lectura siguiente.
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

    /// Agrega a `window` cada cuadro completo.
    pub fn decode(&mut self, bytes: &[u8], window: &mut StereoWindow) {
        self.pending.extend_from_slice(bytes);
        let frame_bytes = self.format.bytes_per_frame();
        let complete = self.pending.len() / frame_bytes * frame_bytes;
        for frame in self.pending[..complete].chunks_exact(frame_bytes) {
            window.push(self.pair(frame));
        }
        self.pending.drain(..complete);
    }

    /// El orden de bytes es el nativo; con un canal, se repite en los dos.
    fn pair(&self, frame: &[u8]) -> SamplePair {
        let samples: Vec<f32> = frame
            .chunks_exact(self.format.bytes_per_sample())
            .map(|sample| f32::from(i16::from_ne_bytes([sample[0], sample[1]])) / SAMPLE_SCALE)
            .collect();
        let left = samples[0];
        let right = samples.last().copied().unwrap_or(left);
        SamplePair { left, right }
    }
}

/// Los últimos `capacity` pares; los viejos se descartan.
#[derive(Debug)]
pub struct StereoWindow {
    pairs: VecDeque<SamplePair>,
    capacity: usize,
}

impl StereoWindow {
    pub fn new(capacity: usize) -> Self {
        Self {
            pairs: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, pair: SamplePair) {
        if self.pairs.len() == self.capacity {
            self.pairs.pop_front();
        }
        self.pairs.push_back(pair);
    }

    /// Sin señal, lo guardado ya no vale: todas las formas pasan a ver silencio.
    pub fn clear(&mut self) {
        self.pairs.clear();
    }

    /// Exactamente `count` pares (como mucho la capacidad), con silencio adelante si
    /// todavía faltan.
    pub fn latest(&self, count: usize) -> Vec<SamplePair> {
        let count = count.min(self.capacity);
        let available = count.min(self.pairs.len());
        let mut result = vec![SamplePair::default(); count - available];
        result.extend(self.pairs.range(self.pairs.len() - available..));
        result
    }
}

/// Promedio de los canales de cada par.
pub fn mono(pairs: &[SamplePair]) -> Vec<f32> {
    pairs.iter().map(|pair| pair.mono()).collect()
}

pub fn left(pairs: &[SamplePair]) -> Vec<f32> {
    pairs.iter().map(|pair| pair.left).collect()
}

pub fn right(pairs: &[SamplePair]) -> Vec<f32> {
    pairs.iter().map(|pair| pair.right).collect()
}

/// Seno de `frequency` Hz en PCM estéreo de 16 bits a 44,1 kHz, `amplitude` entre 0 y 1.
#[cfg(test)]
pub(crate) fn sine_pcm(frequency: f32, amplitude: f32, frames: usize) -> Vec<u8> {
    sine_samples(frequency, amplitude, frames)
        .into_iter()
        .flat_map(|sample| {
            let bytes = ((sample * (SAMPLE_SCALE - 1.0)) as i16).to_ne_bytes();
            [bytes, bytes].concat()
        })
        .collect()
}

/// Seno de `frequency` Hz a 44,1 kHz, en [-amplitude, amplitude].
#[cfg(test)]
pub(crate) fn sine_samples(frequency: f32, amplitude: f32, count: usize) -> Vec<f32> {
    const SAMPLE_RATE: f32 = 44_100.0;
    (0..count)
        .map(|index| {
            let phase = 2.0 * std::f32::consts::PI * frequency * index as f32 / SAMPLE_RATE;
            phase.sin() * amplitude
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo() -> PcmFormat {
        PcmFormat::parse("44100:16:2").unwrap()
    }

    #[test]
    fn decoder_keeps_frames_split_between_reads() {
        let left = 16_384_i16.to_ne_bytes();
        let right = (-16_384_i16).to_ne_bytes();
        let louder = 32_767_i16.to_ne_bytes();
        let bytes = [left, right, louder, louder].concat();
        let mut decoder = PcmDecoder::new(stereo());
        let mut window = StereoWindow::new(2);
        decoder.decode(&bytes[..3], &mut window);
        assert_eq!(window.latest(2), [SamplePair::default(); 2]);
        decoder.decode(&bytes[3..6], &mut window);
        let first = SamplePair {
            left: 0.5,
            right: -0.5,
        };
        assert_eq!(window.latest(2), [SamplePair::default(), first]);
        assert_eq!(first.mono(), 0.0);
        decoder.decode(&bytes[6..], &mut window);
        let latest = window.latest(2);
        assert_eq!(latest[0], first);
        assert!((latest[1].mono() - 1.0).abs() < 0.001);
    }

    #[test]
    fn mono_fifos_repeat_the_sample_in_both_channels() {
        let mut decoder = PcmDecoder::new(PcmFormat::parse("44100:16:1").unwrap());
        let mut window = StereoWindow::new(1);
        decoder.decode(&16_384_i16.to_ne_bytes(), &mut window);
        let pair = window.latest(1)[0];
        assert_eq!((pair.left, pair.right), (0.5, 0.5));
    }

    #[test]
    fn the_window_keeps_only_the_latest_pairs() {
        let mut window = StereoWindow::new(3);
        for value in [1.0, 2.0, 3.0, 4.0] {
            window.push(SamplePair {
                left: value,
                right: value,
            });
        }
        assert_eq!(left(&window.latest(3)), [2.0, 3.0, 4.0]);
        assert_eq!(left(&window.latest(2)), [3.0, 4.0]);
        assert_eq!(left(&window.latest(5)), [2.0, 3.0, 4.0]);
        window.clear();
        assert_eq!(right(&window.latest(2)), [0.0, 0.0]);
    }
}
