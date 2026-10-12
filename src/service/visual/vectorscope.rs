//! Vectorscopio: cada par L/R como punto en ejes M/S, con escala automática.

use std::time::Duration;

use super::levels::correlation;
use super::pcm::SamplePair;
use crate::domain::ScopePoint;

/// Pares que se dibujan por cuadro.
pub const VECTORSCOPE_FRAMES: usize = 2048;
/// Fracción de la escala que se pierde por segundo cuando la señal baja.
pub const VECTORSCOPE_GAIN_DECAY: f32 = 0.5;
/// Debajo de esta amplitud se considera silencio y no hay puntos.
const SILENCE_AMPLITUDE: f32 = 1e-4;

/// `M = (L + R) / 2`, `S = (R − L) / 2`.
fn mid_side(pair: SamplePair) -> ScopePoint {
    ScopePoint {
        side: (pair.right - pair.left) / 2.0,
        mid: (pair.left + pair.right) / 2.0,
    }
}

/// La escala sigue al máximo reciente y vuelve despacio, para que un tema bajo no
/// quede como un punto.
#[derive(Debug, Default)]
pub struct Vectorscope {
    scale: f32,
}

impl Vectorscope {
    /// Puntos ya escalados a [-1, 1] y correlación de los pares.
    pub fn update(&mut self, pairs: &[SamplePair], elapsed: Duration) -> (Vec<ScopePoint>, f32) {
        let points: Vec<ScopePoint> = pairs.iter().map(|pair| mid_side(*pair)).collect();
        let loudest = points
            .iter()
            .map(|point| point.side.abs().max(point.mid.abs()))
            .fold(0.0, f32::max);
        let decayed = self.scale * (1.0 - VECTORSCOPE_GAIN_DECAY * elapsed.as_secs_f32()).max(0.0);
        self.scale = loudest.max(decayed);
        if loudest < SILENCE_AMPLITUDE {
            return (Vec::new(), correlation(pairs));
        }
        let scaled = points
            .into_iter()
            .map(|point| ScopePoint {
                side: (point.side / self.scale).clamp(-1.0, 1.0),
                mid: (point.mid / self.scale).clamp(-1.0, 1.0),
            })
            .collect();
        (scaled, correlation(pairs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::visual::pcm::sine_samples;

    const FRAME: Duration = Duration::from_millis(50);

    fn pairs(sign: f32) -> Vec<SamplePair> {
        sine_samples(441.0, 0.25, VECTORSCOPE_FRAMES)
            .into_iter()
            .map(|sample| SamplePair {
                left: sample,
                right: sign * sample,
            })
            .collect()
    }

    #[test]
    fn equal_channels_are_a_vertical_line() {
        let (points, correlation) = Vectorscope::default().update(&pairs(1.0), FRAME);
        assert!(points.iter().all(|point| point.side == 0.0));
        assert!(
            points
                .iter()
                .any(|point| (point.mid.abs() - 1.0).abs() < 0.01)
        );
        assert!((correlation - 1.0).abs() < 1e-4);
    }

    #[test]
    fn an_inverted_channel_is_a_horizontal_line() {
        let (points, correlation) = Vectorscope::default().update(&pairs(-1.0), FRAME);
        assert!(points.iter().all(|point| point.mid == 0.0));
        assert!((correlation + 1.0).abs() < 1e-4);
    }

    #[test]
    fn silence_has_no_points() {
        let silence = vec![SamplePair::default(); VECTORSCOPE_FRAMES];
        let (points, correlation) = Vectorscope::default().update(&silence, FRAME);
        assert!(points.is_empty());
        assert_eq!(correlation, 0.0);
    }

    #[test]
    fn the_scale_comes_back_slowly() {
        let mut scope = Vectorscope::default();
        let loud = pairs(1.0);
        scope.update(&loud, FRAME);
        let quiet: Vec<SamplePair> = loud
            .iter()
            .map(|pair| SamplePair {
                left: pair.left / 2.0,
                right: pair.right / 2.0,
            })
            .collect();
        let (points, _) = scope.update(&quiet, FRAME);
        let loudest = points
            .iter()
            .map(|point| point.mid.abs())
            .fold(0.0, f32::max);
        assert!(loudest > 0.5 && loudest < 0.6, "{loudest}");
    }
}
