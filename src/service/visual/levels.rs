//! Niveles de una señal: RMS, pico, decibeles y correlación entre canales.

use super::pcm::SamplePair;

/// Evita el logaritmo de cero.
const AMPLITUDE_EPSILON: f32 = 1e-9;
const DECIBELS_PER_LOG: f32 = 20.0;

/// `20·log10(amplitud)`.
pub fn decibels(amplitude: f32) -> f32 {
    DECIBELS_PER_LOG * (amplitude + AMPLITUDE_EPSILON).log10()
}

/// Raíz del promedio de los cuadrados; 0 sin muestras.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let squares: f32 = samples.iter().map(|sample| sample * sample).sum();
    (squares / samples.len() as f32).sqrt()
}

/// Máximo valor absoluto.
pub fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0, f32::max)
}

/// `Σ L·R / √(Σ L² · Σ R²)`, de -1 a +1. Con silencio en algún canal vale 0.
pub fn correlation(pairs: &[SamplePair]) -> f32 {
    let (mut product, mut left_energy, mut right_energy) = (0.0_f32, 0.0_f32, 0.0_f32);
    for pair in pairs {
        product += pair.left * pair.right;
        left_energy += pair.left * pair.left;
        right_energy += pair.right * pair.right;
    }
    let energy = (left_energy * right_energy).sqrt();
    if energy <= 0.0 {
        return 0.0;
    }
    (product / energy).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::visual::pcm::sine_samples;

    /// Un número entero de ciclos de 441 Hz a 44,1 kHz.
    const CYCLES: usize = 4_410;

    #[test]
    fn a_full_scale_sine_peaks_at_zero_and_has_rms_minus_three() {
        let sine = sine_samples(441.0, 1.0, CYCLES);
        assert!(decibels(peak(&sine)).abs() < 0.01);
        assert!((decibels(rms(&sine)) + 3.01).abs() < 0.01);
        let half = sine_samples(441.0, 0.5, CYCLES);
        assert!((decibels(peak(&half)) + 6.02).abs() < 0.01);
        assert!((decibels(rms(&half)) + 9.03).abs() < 0.01);
    }

    #[test]
    fn correlation_goes_from_minus_one_to_one() {
        let sine = sine_samples(441.0, 0.5, CYCLES);
        let pairs = |sign: f32| -> Vec<SamplePair> {
            sine.iter()
                .map(|sample| SamplePair {
                    left: *sample,
                    right: sign * sample,
                })
                .collect()
        };
        assert!((correlation(&pairs(1.0)) - 1.0).abs() < 1e-4);
        assert!((correlation(&pairs(-1.0)) + 1.0).abs() < 1e-4);
        assert_eq!(correlation(&pairs(0.0)), 0.0);
        assert_eq!(correlation(&[]), 0.0);
        assert_eq!(rms(&[]), 0.0);
    }
}
