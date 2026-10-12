//! Medidores: RMS y pico retenido por canal, en dBFS.

use std::time::Duration;

use super::levels::{decibels, peak, rms};
use super::peak::{HeldPeak, PeakDecay};
use crate::domain::{ChannelLevel, METER_FLOOR_DB, METER_PEAK_HOLD};

/// El pico retenido se sostiene `METER_PEAK_HOLD` y después cae en dB por segundo.
pub const METER_PEAK_DECAY: PeakDecay = PeakDecay {
    hold: METER_PEAK_HOLD,
    fall_per_second: 20.0,
};

#[derive(Debug, Default)]
pub struct Meters {
    peaks: Vec<HeldPeak>,
}

impl Meters {
    /// `channels` tiene las muestras de cada canal en la ventana del RMS.
    pub fn update(&mut self, channels: &[Vec<f32>], elapsed: Duration) -> Vec<ChannelLevel> {
        if self.peaks.len() != channels.len() {
            self.peaks = vec![HeldPeak::new(METER_FLOOR_DB); channels.len()];
        }
        channels
            .iter()
            .zip(&mut self.peaks)
            .map(|(samples, held)| {
                held.update(floored(decibels(peak(samples))), elapsed, METER_PEAK_DECAY);
                ChannelLevel {
                    rms: floored(decibels(rms(samples))),
                    peak: held.value(),
                }
            })
            .collect()
    }
}

/// Nada debajo del piso de la escala.
fn floored(decibels: f32) -> f32 {
    decibels.max(METER_FLOOR_DB)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::visual::pcm::sine_samples;

    const FRAME: Duration = Duration::from_millis(50);

    #[test]
    fn a_full_scale_sine_reads_zero_peak_and_minus_three_rms() {
        let sine = sine_samples(441.0, 1.0, 4_410);
        let levels = Meters::default().update(&[sine.clone(), sine], FRAME);
        assert_eq!(levels.len(), 2);
        assert!(levels[0].peak.abs() < 0.01);
        assert!((levels[0].rms + 3.01).abs() < 0.01);
    }

    #[test]
    fn the_peak_holds_and_then_falls() {
        let mut meters = Meters::default();
        let sine = sine_samples(441.0, 1.0, 4_410);
        meters.update(&[sine], FRAME);
        let silence = vec![0.0; 4_410];
        let held = meters.update(std::slice::from_ref(&silence), Duration::from_millis(1_400));
        assert!(held[0].peak.abs() < 0.01);
        assert_eq!(held[0].rms, METER_FLOOR_DB);
        meters.update(std::slice::from_ref(&silence), Duration::from_millis(200));
        let falling = meters.update(std::slice::from_ref(&silence), Duration::from_millis(500));
        assert!(falling[0].peak < -5.0 && falling[0].peak > METER_FLOOR_DB);
        let gone = meters.update(&[silence], Duration::from_secs(10));
        assert_eq!(gone[0], ChannelLevel::SILENT);
    }
}
