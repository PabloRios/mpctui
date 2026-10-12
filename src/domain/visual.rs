//! Visualizador: sus seis formas, lo que la TUI le pide y el cuadro listo para dibujar.

use std::time::Duration;

/// Canales de una señal estéreo.
pub const STEREO_CHANNELS: u8 = 2;
/// La Cascada avanza estas columnas por segundo.
pub const WATERFALL_COLUMNS_PER_SECOND: u32 = 10;
/// Ventana del RMS de los Medidores.
pub const RMS_WINDOW: Duration = Duration::from_millis(300);
/// Cuánto se sostiene el pico de los Medidores antes de caer.
pub const METER_PEAK_HOLD: Duration = Duration::from_millis(1_500);
/// Piso de la escala de los Medidores, en dBFS: más bajo se muestra vacío.
pub const METER_FLOOR_DB: f32 = -60.0;

/// Forma del visualizador, en el orden del ciclo de la tecla `6`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualizerStyle {
    #[default]
    Bars,
    Mirror,
    Wave,
    Waterfall,
    Vectorscope,
    Meters,
}

impl VisualizerStyle {
    pub const ALL: [VisualizerStyle; 6] = [
        VisualizerStyle::Bars,
        VisualizerStyle::Mirror,
        VisualizerStyle::Wave,
        VisualizerStyle::Waterfall,
        VisualizerStyle::Vectorscope,
        VisualizerStyle::Meters,
    ];

    /// Espejo y Vectorscopio comparan los dos canales.
    pub fn needs_stereo(self) -> bool {
        matches!(self, VisualizerStyle::Mirror | VisualizerStyle::Vectorscope)
    }

    pub fn is_available(self, channels: u8) -> bool {
        channels >= STEREO_CHANNELS || !self.needs_stereo()
    }

    /// Las formas que se pueden mostrar con `channels` canales, en el orden del ciclo.
    pub fn available(channels: u8) -> Vec<VisualizerStyle> {
        Self::ALL
            .into_iter()
            .filter(|style| style.is_available(channels))
            .collect()
    }

    /// La forma siguiente del ciclo; con un canal salta las que necesitan dos.
    pub fn next(self, channels: u8) -> Self {
        let available = Self::available(channels);
        match available.iter().position(|style| *style == self) {
            Some(index) => available[(index + 1) % available.len()],
            None => available[0],
        }
    }
}

/// Qué forma calcular y con qué resolución. La TUI lo decide según el área.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisualRequest {
    pub style: VisualizerStyle,
    /// Bandas (Barras, Espejo) o columnas de datos (Onda, Cascada).
    pub columns: usize,
    /// Bandas de la Cascada, una por fila.
    pub rows: usize,
}

/// Niveles por banda, entre 0 y 1, con el máximo reciente de cada una.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BandFrame {
    pub levels: Vec<f32>,
    pub peaks: Vec<f32>,
}

impl BandFrame {
    /// Ya no queda nada que dibujar: ni barras ni picos.
    pub fn is_flat(&self) -> bool {
        self.levels
            .iter()
            .chain(&self.peaks)
            .all(|level| *level <= 0.0)
    }
}

/// Mínimo y máximo de las muestras que cubre una columna de la Onda, entre -1 y 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SampleRange {
    pub low: f32,
    pub high: f32,
}

/// Un punto del Vectorscopio, ya escalado a [-1, 1]: lado (`S`) y centro (`M`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScopePoint {
    pub side: f32,
    pub mid: f32,
}

/// Nivel de un canal de los Medidores, en dBFS, nunca debajo de `METER_FLOOR_DB`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelLevel {
    pub rms: f32,
    /// Pico retenido.
    pub peak: f32,
}

impl ChannelLevel {
    pub const SILENT: ChannelLevel = ChannelLevel {
        rms: METER_FLOOR_DB,
        peak: METER_FLOOR_DB,
    };
}

/// Lo que dibuja cada forma.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualContent {
    Bars(BandFrame),
    Mirror {
        left: BandFrame,
        right: BandFrame,
    },
    Wave {
        columns: Vec<SampleRange>,
        /// La ventana empieza en un cruce por cero ascendente.
        synced: bool,
        /// Tiempo que cubre la ventana.
        span: Duration,
    },
    /// Una columna por instante, de la más vieja a la más nueva; una banda por fila.
    Waterfall {
        columns: Vec<Vec<f32>>,
    },
    Vectorscope {
        points: Vec<ScopePoint>,
        correlation: f32,
    },
    /// Sin correlación con un solo canal.
    Meters {
        channels: Vec<ChannelLevel>,
        correlation: Option<f32>,
    },
}

impl VisualContent {
    pub fn style(&self) -> VisualizerStyle {
        match self {
            VisualContent::Bars(_) => VisualizerStyle::Bars,
            VisualContent::Mirror { .. } => VisualizerStyle::Mirror,
            VisualContent::Wave { .. } => VisualizerStyle::Wave,
            VisualContent::Waterfall { .. } => VisualizerStyle::Waterfall,
            VisualContent::Vectorscope { .. } => VisualizerStyle::Vectorscope,
            VisualContent::Meters { .. } => VisualizerStyle::Meters,
        }
    }

    /// No queda nada que dibujar.
    pub fn is_flat(&self) -> bool {
        match self {
            VisualContent::Bars(bands) => bands.is_flat(),
            VisualContent::Mirror { left, right } => left.is_flat() && right.is_flat(),
            VisualContent::Wave { columns, .. } => columns
                .iter()
                .all(|range| range.low == 0.0 && range.high == 0.0),
            VisualContent::Waterfall { columns } => {
                columns.iter().flatten().all(|level| *level <= 0.0)
            }
            VisualContent::Vectorscope { points, .. } => points.is_empty(),
            VisualContent::Meters { channels, .. } => channels
                .iter()
                .all(|level| level.rms <= METER_FLOOR_DB && level.peak <= METER_FLOOR_DB),
        }
    }
}

/// Un cuadro del visualizador.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualFrame {
    pub content: VisualContent,
    /// `false` si no llegan datos (pausado, detenido o sin escritor).
    pub signal: bool,
}

/// Antes del primer cuadro: nada que dibujar.
impl Default for VisualFrame {
    fn default() -> Self {
        Self {
            content: VisualContent::Bars(BandFrame::default()),
            signal: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cycle_returns_to_bars_after_meters() {
        let mut style = VisualizerStyle::Bars;
        let mut seen = vec![style];
        for _ in 1..VisualizerStyle::ALL.len() {
            style = style.next(STEREO_CHANNELS);
            seen.push(style);
        }
        assert_eq!(seen, VisualizerStyle::ALL);
        assert_eq!(style.next(STEREO_CHANNELS), VisualizerStyle::Bars);
    }

    #[test]
    fn mono_skips_the_stereo_styles() {
        let mono = 1;
        assert_eq!(
            VisualizerStyle::available(mono),
            [
                VisualizerStyle::Bars,
                VisualizerStyle::Wave,
                VisualizerStyle::Waterfall,
                VisualizerStyle::Meters,
            ]
        );
        assert_eq!(VisualizerStyle::Bars.next(mono), VisualizerStyle::Wave);
        assert_eq!(
            VisualizerStyle::Waterfall.next(mono),
            VisualizerStyle::Meters
        );
        assert_eq!(VisualizerStyle::Mirror.next(mono), VisualizerStyle::Bars);
    }

    #[test]
    fn flat_content_has_nothing_to_draw() {
        assert!(VisualFrame::default().content.is_flat());
        let bands = BandFrame {
            levels: vec![0.0, 0.0],
            peaks: vec![0.0, 0.2],
        };
        assert!(!VisualContent::Bars(bands).is_flat());
        let meters = VisualContent::Meters {
            channels: vec![ChannelLevel::SILENT],
            correlation: None,
        };
        assert!(meters.is_flat());
        let scope = VisualContent::Vectorscope {
            points: Vec::new(),
            correlation: 0.0,
        };
        assert!(scope.is_flat());
    }
}
