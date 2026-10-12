//! Cómo se ve la interfaz (glifos, idioma, barras). Lo arma el composition root a
//! partir de la configuración.

use super::glyphs::Glyphs;
use crate::domain::VisualizerStyle;
use crate::i18n::Catalog;

#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    pub glyphs: &'static Glyphs,
    /// Textos en el idioma configurado.
    pub texts: &'static dyn Catalog,
    pub progress_bars: ProgressBars,
    /// Forma con la que arranca el visualizador.
    pub visualizer_style: VisualizerStyle,
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
