//! Modo de layout según el tamaño de la terminal (completo, compacto o aviso).

pub const FULL_MIN_WIDTH: u16 = 100;
pub const FULL_MIN_HEIGHT: u16 = 28;
pub const COMPACT_MIN_WIDTH: u16 = 60;
pub const COMPACT_MIN_HEIGHT: u16 = 16;
/// Por debajo de este ancho las tablas de temas no muestran la columna Artista.
pub const ARTIST_COLUMN_MIN_WIDTH: u16 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Full,
    Compact,
    TooSmall,
}

/// Se decide en cada dibujado a partir del tamaño del frame.
pub fn layout_mode(width: u16, height: u16) -> LayoutMode {
    if width >= FULL_MIN_WIDTH && height >= FULL_MIN_HEIGHT {
        LayoutMode::Full
    } else if width >= COMPACT_MIN_WIDTH && height >= COMPACT_MIN_HEIGHT {
        LayoutMode::Compact
    } else {
        LayoutMode::TooSmall
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_pick_the_mode() {
        assert_eq!(layout_mode(100, 30), LayoutMode::Full);
        assert_eq!(layout_mode(100, 28), LayoutMode::Full);
        assert_eq!(layout_mode(140, 27), LayoutMode::Compact);
        assert_eq!(layout_mode(72, 16), LayoutMode::Compact);
        assert_eq!(layout_mode(59, 40), LayoutMode::TooSmall);
        assert_eq!(layout_mode(40, 10), LayoutMode::TooSmall);
    }
}
