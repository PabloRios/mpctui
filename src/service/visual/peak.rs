//! Pico retenido: sube con el valor, se queda quieto un rato y después cae.

use std::time::Duration;

/// Cuánto se sostiene un pico y a qué velocidad cae después (en unidades por segundo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakDecay {
    pub hold: Duration,
    pub fall_per_second: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeldPeak {
    value: f32,
    /// Desde cuándo está quieto.
    age: Duration,
}

impl HeldPeak {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            age: Duration::ZERO,
        }
    }

    pub fn value(self) -> f32 {
        self.value
    }

    /// Sube con `current`; si no, espera `hold` y después cae, sin bajar de `current`.
    pub fn update(&mut self, current: f32, elapsed: Duration, decay: PeakDecay) {
        if current >= self.value {
            *self = Self::new(current);
        } else if self.age < decay.hold {
            self.age += elapsed;
        } else {
            let fallen = self.value - decay.fall_per_second * elapsed.as_secs_f32();
            self.value = fallen.max(current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECAY: PeakDecay = PeakDecay {
        hold: Duration::from_millis(500),
        fall_per_second: 1.0,
    };

    #[test]
    fn peaks_hold_and_then_fall_without_passing_the_value() {
        let mut peak = HeldPeak::new(0.0);
        peak.update(1.0, Duration::ZERO, DECAY);
        peak.update(0.0, Duration::from_millis(400), DECAY);
        peak.update(0.0, Duration::from_millis(200), DECAY);
        assert_eq!(peak.value(), 1.0);
        peak.update(0.0, Duration::from_millis(200), DECAY);
        assert!((peak.value() - 0.8).abs() < 1e-6);
        peak.update(0.7, Duration::from_secs(5), DECAY);
        assert_eq!(peak.value(), 0.7);
    }
}
