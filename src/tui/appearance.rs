//! Cómo se ve la interfaz (glifos, idioma, colores, barras). Lo arma el composition
//! root a partir de la configuración.

use super::glyphs::Glyphs;
use super::theme::Theme;
use crate::domain::VisualizerStyle;
use crate::i18n::Catalog;

#[derive(Debug, Clone)]
pub struct Appearance {
    pub glyphs: &'static Glyphs,
    /// Textos en el idioma configurado.
    pub texts: &'static dyn Catalog,
    /// Tema de colores ya resuelto para la terminal; `t` pasa al siguiente.
    pub theme: Theme,
    pub progress_bars: ProgressBars,
    /// Forma con la que arranca el visualizador.
    pub visualizer_style: VisualizerStyle,
    /// Línea actual de la letra en el panel "ahora suena".
    pub lyrics_line: bool,
}

/// Barras opcionales del panel "ahora suena". La del tema siempre se muestra.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProgressBars {
    /// Un segmento por tema del álbum en curso.
    pub album: bool,
    /// Un segmento por álbum de la cola.
    pub queue: bool,
}

impl ProgressBars {
    /// Cuántas filas suman al panel.
    pub fn count(self) -> u16 {
        u16::from(self.album) + u16::from(self.queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_enabled_bars() {
        assert_eq!(ProgressBars::default().count(), 0);
        let both = ProgressBars {
            album: true,
            queue: true,
        };
        assert_eq!(both.count(), 2);
    }
}
