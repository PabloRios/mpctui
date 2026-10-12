//! Espectro de audio: bandas logarítmicas y el cuadro que se dibuja.

/// Rango de frecuencias que cubren las bandas.
pub const SPECTRUM_MIN_HZ: f32 = 40.0;
pub const SPECTRUM_MAX_HZ: f32 = 16_000.0;

/// Niveles por banda, entre 0 y 1.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpectrumFrame {
    pub levels: Vec<f32>,
    /// Máximo reciente de cada banda.
    pub peaks: Vec<f32>,
    /// `false` si no llegan datos (pausado, detenido o sin escritor).
    pub signal: bool,
}

impl SpectrumFrame {
    /// Todo en cero y sin señal.
    pub fn silent(bands: usize) -> Self {
        Self {
            levels: vec![0.0; bands],
            peaks: vec![0.0; bands],
            signal: false,
        }
    }

    /// Ya no queda nada que dibujar: ni barras ni picos.
    pub fn is_flat(&self) -> bool {
        self.levels
            .iter()
            .chain(&self.peaks)
            .all(|level| *level <= 0.0)
    }
}

/// `bands + 1` bordes en escala logarítmica: `min × (max/min)^(i/bandas)`.
pub fn band_edges(bands: usize) -> Vec<f32> {
    let ratio = SPECTRUM_MAX_HZ / SPECTRUM_MIN_HZ;
    (0..=bands)
        .map(|index| SPECTRUM_MIN_HZ * ratio.powf(index as f32 / bands as f32))
        .collect()
}

/// Centro (geométrico) de cada banda.
pub fn band_centers(bands: usize) -> Vec<f32> {
    band_edges(bands)
        .windows(2)
        .map(|edge| (edge[0] * edge[1]).sqrt())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_are_logarithmic() {
        let edges = band_edges(4);
        assert_eq!(edges.len(), 5);
        assert!((edges[0] - SPECTRUM_MIN_HZ).abs() < 0.01);
        assert!((edges[4] - SPECTRUM_MAX_HZ).abs() < 1.0);
        let ratios: Vec<f32> = edges.windows(2).map(|edge| edge[1] / edge[0]).collect();
        assert!(
            ratios
                .windows(2)
                .all(|pair| (pair[0] - pair[1]).abs() < 0.001)
        );
        let centers = band_centers(4);
        assert!(centers[0] > edges[0] && centers[0] < edges[1]);
    }

    #[test]
    fn silent_frames_are_flat() {
        assert!(SpectrumFrame::silent(8).is_flat());
        let frame = SpectrumFrame {
            peaks: vec![0.0, 0.2],
            ..SpectrumFrame::silent(2)
        };
        assert!(!frame.is_flat());
    }
}
