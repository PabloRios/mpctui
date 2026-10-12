//! Onda (osciloscopio): una ventana de muestras que arranca en un cruce por cero
//! ascendente y, por columna, el segmento entre su mínimo y su máximo.

use crate::domain::SampleRange;

/// Frames que muestra la Onda: unos 23 ms a 44,1 kHz.
pub const WAVE_FRAMES: usize = 1024;
/// El cruce se busca en la primera mitad de un búfer del doble de la ventana.
pub const WAVE_BUFFER_FRAMES: usize = 2 * WAVE_FRAMES;
/// Escala fija: un tema bajo se ve chico a propósito.
pub const WAVE_GAIN: f32 = 1.0;

/// Columnas de la Onda y si quedó sincronizada. `samples` son las últimas muestras
/// mono (idealmente `WAVE_BUFFER_FRAMES`).
pub fn wave(samples: &[f32], columns: usize) -> (Vec<SampleRange>, bool) {
    let search_end = samples.len().saturating_sub(WAVE_FRAMES);
    let crossing = rising_zero_crossing(&samples[..search_end.min(samples.len())]);
    let start = crossing.unwrap_or_default();
    let end = (start + WAVE_FRAMES).min(samples.len());
    (
        column_ranges(&samples[start..end], columns),
        crossing.is_some(),
    )
}

/// Primer `i` con `muestra[i−1] < 0 ≤ muestra[i]`.
fn rising_zero_crossing(samples: &[f32]) -> Option<usize> {
    samples
        .windows(2)
        .position(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|index| index + 1)
}

/// Mínimo y máximo de cada tramo. Cada tramo incluye la última muestra del anterior,
/// así la línea queda continua aunque la señal salte entre columnas.
fn column_ranges(samples: &[f32], columns: usize) -> Vec<SampleRange> {
    if samples.is_empty() {
        return vec![SampleRange::default(); columns];
    }
    (0..columns)
        .map(|column| {
            let start = column * samples.len() / columns;
            let end = ((column + 1) * samples.len() / columns).max(start + 1);
            let span = &samples[start.saturating_sub(1)..end.min(samples.len())];
            let scaled = |sample: f32| (sample * WAVE_GAIN).clamp(-1.0, 1.0);
            SampleRange {
                low: scaled(span.iter().copied().fold(f32::INFINITY, f32::min)),
                high: scaled(span.iter().copied().fold(f32::NEG_INFINITY, f32::max)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::visual::pcm::sine_samples;

    const COLUMNS: usize = 100;
    /// Margen por una muestra de diferencia en el arranque.
    const TOLERANCE: f32 = 0.05;

    #[test]
    fn columns_cover_the_minimum_and_maximum_of_their_span() {
        let samples = [0.0, 0.5, -0.5, 1.0, -1.0, 0.25];
        let ranges = column_ranges(&samples, 3);
        assert_eq!(
            ranges[0],
            SampleRange {
                low: 0.0,
                high: 0.5
            }
        );
        assert_eq!(
            ranges[1],
            SampleRange {
                low: -0.5,
                high: 1.0
            }
        );
        assert_eq!(
            ranges[2],
            SampleRange {
                low: -1.0,
                high: 1.0
            }
        );
    }

    #[test]
    fn the_rising_crossing_is_found() {
        assert_eq!(rising_zero_crossing(&[0.5, -0.2, -0.1, 0.0, 0.3]), Some(3));
        assert_eq!(rising_zero_crossing(&[0.5, 0.2, -0.1]), None);
    }

    #[test]
    fn a_sine_looks_the_same_in_successive_frames() {
        let sine = sine_samples(441.0, 0.8, WAVE_BUFFER_FRAMES * 2);
        let (first, synced) = wave(&sine[..WAVE_BUFFER_FRAMES], COLUMNS);
        assert!(synced);
        let (later, synced) = wave(&sine[37..37 + WAVE_BUFFER_FRAMES], COLUMNS);
        assert!(synced);
        for (before, after) in first.iter().zip(&later) {
            assert!(
                (before.low - after.low).abs() < TOLERANCE,
                "{before:?} {after:?}"
            );
            assert!(
                (before.high - after.high).abs() < TOLERANCE,
                "{before:?} {after:?}"
            );
        }
    }

    #[test]
    fn without_crossings_the_wave_runs_free() {
        let noise = vec![0.3; WAVE_BUFFER_FRAMES];
        let (columns, synced) = wave(&noise, COLUMNS);
        assert!(!synced);
        assert_eq!(columns.len(), COLUMNS);
        let (silence, synced) = wave(&[0.0; WAVE_BUFFER_FRAMES], COLUMNS);
        assert!(!synced);
        assert!(silence.iter().all(|range| *range == SampleRange::default()));
    }
}
