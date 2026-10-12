//! Juegos de glifos: Unicode por defecto y ASCII como fallback, con el mismo layout.
//! Es el único lugar con glifos literales; las vistas los piden por su rol.

use ratatui::symbols::border;

/// Uniones entre paneles; cada nombre indica hacia dónde sale la línea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Junctions {
    /// `├`: línea vertical con salida a la derecha.
    pub tee_right: &'static str,
    /// `┤`: línea vertical con salida a la izquierda.
    pub tee_left: &'static str,
    /// `┬`: línea horizontal con salida hacia abajo.
    pub tee_down: &'static str,
    /// `┴`: línea horizontal con salida hacia arriba.
    pub tee_up: &'static str,
    pub cross: &'static str,
}

/// Cuadros del spinner de reconexión.
pub const SPINNER_FRAMES: usize = 4;
/// Primer carácter braille (sin puntos). Un punto encendido suma su bit: es el único
/// glifo que se calcula en lugar de escribirse.
pub const BRAILLE_BASE: u32 = 0x2800;

/// Cómo se dibujan los puntos de la Onda y del Vectorscopio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plot {
    /// Puntos braille de 2 × 4 por celda (`BRAILLE_BASE` + bits).
    Braille,
    /// Una celda de texto por columna, con sub-niveles verticales.
    Cells(CellPlot),
}

/// Puntos en celdas de texto (juego ASCII).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPlot {
    /// Sub-niveles de una celda, de arriba a abajo.
    pub levels: &'static [&'static str],
    /// Une un salto vertical que ocupa más de un sub-nivel.
    pub join: &'static str,
    /// Guías del Vectorscopio hacia L (arriba a la izquierda) y hacia R.
    pub guide_left: &'static str,
    pub guide_right: &'static str,
    /// Densidad de puntos del Vectorscopio por celda: vacío y después de menos a más.
    pub density: &'static [&'static str],
}

/// Flechas del teclado, para nombrarlas en la ayuda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrowKeys {
    pub up: &'static str,
    pub down: &'static str,
    pub left: &'static str,
    pub right: &'static str,
}

/// Un glifo por rol. Ver la tabla 2.1 de `DESIGN.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyphs {
    pub play: &'static str,
    pub pause: &'static str,
    pub stop: &'static str,
    pub now_playing: &'static str,
    pub cursor: &'static str,
    pub directory: &'static str,
    pub playlist: &'static str,
    pub bar_filled: &'static str,
    pub bar_head: &'static str,
    pub bar_empty: &'static str,
    pub bar_separator: &'static str,
    pub bar_open: &'static str,
    pub bar_close: &'static str,
    pub bar_unknown: &'static str,
    pub bar_more: &'static str,
    pub meter_filled: &'static str,
    pub meter_empty: &'static str,
    pub flag_on: &'static str,
    pub flag_off: &'static str,
    pub flag_once: &'static str,
    /// Entre la tecla de un modo y su indicador (`z ●` / `z[x]`).
    pub flag_key_gap: &'static str,
    pub group_rule: &'static str,
    pub breadcrumb: &'static str,
    pub ellipsis: &'static str,
    pub scroll_up: &'static str,
    pub scroll_down: &'static str,
    pub scroll_track: &'static str,
    pub scroll_thumb: &'static str,
    pub connected: &'static str,
    pub disconnected: &'static str,
    pub spinner: [&'static str; SPINNER_FRAMES],
    /// Sub-celdas de las barras del espectro: vacío y después de menor a mayor. La
    /// cantidad de niveles por celda sale de la longitud.
    pub spectrum_levels: &'static [&'static str],
    pub spectrum_peak: &'static str,
    /// Medias celdas del Espejo: el canal L crece hacia arriba y el R hacia abajo.
    pub mirror_up: &'static [&'static str],
    pub mirror_down: &'static [&'static str],
    pub plot: Plot,
    /// Sombreado de la Cascada: vacío y después de menos a más intensidad.
    pub shades: &'static [&'static str],
    /// Fracciones horizontales de las barras de los Medidores, de menor a celda llena.
    pub meter_eighths: &'static [&'static str],
    pub meter_peak: &'static str,
    pub scale_tick: &'static str,
    pub correlation_center: &'static str,
    pub correlation_marker: &'static str,
    /// Indicador de la forma activa del visualizador.
    pub page_active: &'static str,
    pub page_inactive: &'static str,
    pub message_info: &'static str,
    pub message_error: &'static str,
    /// Cursor al final del texto que se está escribiendo.
    pub input_cursor: &'static str,
    /// Flechas del teclado en la ayuda. Sin ellas (ASCII), se nombran con palabras
    /// del idioma.
    pub arrow_keys: Option<ArrowKeys>,
    pub border: border::Set<'static>,
    pub junction: Junctions,
}

pub const UNICODE: Glyphs = Glyphs {
    play: "▶",
    pause: "⏸",
    stop: "■",
    now_playing: "♪",
    cursor: "▸",
    directory: "▪",
    playlist: "≡",
    bar_filled: "━",
    bar_head: "●",
    bar_empty: "─",
    bar_separator: "┃",
    bar_open: "",
    bar_close: "",
    bar_unknown: "╌",
    bar_more: "…",
    meter_filled: "■",
    meter_empty: "□",
    flag_on: "●",
    flag_off: "○",
    flag_once: "◐",
    flag_key_gap: " ",
    group_rule: "─",
    breadcrumb: " › ",
    ellipsis: "…",
    scroll_up: "▲",
    scroll_down: "▼",
    scroll_track: "│",
    scroll_thumb: "█",
    connected: "●",
    disconnected: "✖",
    spinner: ["◐", "◓", "◑", "◒"],
    spectrum_levels: &[" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"],
    spectrum_peak: "▔",
    mirror_up: &[" ", "▄", "█"],
    mirror_down: &[" ", "▀", "█"],
    plot: Plot::Braille,
    shades: &[" ", "░", "▒", "▓", "█"],
    meter_eighths: &["▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"],
    meter_peak: "│",
    scale_tick: "┴",
    correlation_center: "┼",
    correlation_marker: "●",
    page_active: "●",
    page_inactive: "○",
    message_info: "·",
    message_error: "✖",
    input_cursor: "_",
    arrow_keys: Some(ArrowKeys {
        up: "↑",
        down: "↓",
        left: "←",
        right: "→",
    }),
    border: border::PLAIN,
    junction: Junctions {
        tee_right: "├",
        tee_left: "┤",
        tee_down: "┬",
        tee_up: "┴",
        cross: "┼",
    },
};

const ASCII_CORNER: &str = "+";

pub const ASCII: Glyphs = Glyphs {
    play: "|>",
    pause: "||",
    stop: "[]",
    now_playing: "~",
    cursor: ">",
    directory: "+",
    playlist: "=",
    bar_filled: "=",
    bar_head: ">",
    bar_empty: "-",
    bar_separator: "|",
    bar_open: "[",
    bar_close: "]",
    bar_unknown: "~",
    bar_more: "..",
    meter_filled: "#",
    meter_empty: "-",
    flag_on: "x",
    flag_off: " ",
    flag_once: "1",
    flag_key_gap: "",
    group_rule: "=",
    breadcrumb: " > ",
    ellipsis: "...",
    scroll_up: "^",
    scroll_down: "v",
    scroll_track: ":",
    scroll_thumb: "#",
    connected: "(*)",
    disconnected: "(x)",
    spinner: ["|", "/", "-", "\\"],
    spectrum_levels: &[" ", ".", ":", "#"],
    spectrum_peak: "-",
    mirror_up: &[" ", ".", "#"],
    mirror_down: &[" ", "'", "#"],
    plot: Plot::Cells(CellPlot {
        levels: &["'", "-", "."],
        join: "|",
        guide_left: "\\",
        guide_right: "/",
        density: &[" ", ".", "+", "#"],
    }),
    shades: &[" ", ".", ":", "-", "=", "+", "*", "#", "%", "@"],
    meter_eighths: &["#"],
    meter_peak: "|",
    scale_tick: "+",
    correlation_center: "+",
    correlation_marker: "o",
    page_active: "*",
    page_inactive: ".",
    message_info: "(i)",
    message_error: "(x)",
    input_cursor: "_",
    arrow_keys: None,
    border: border::Set {
        top_left: ASCII_CORNER,
        top_right: ASCII_CORNER,
        bottom_left: ASCII_CORNER,
        bottom_right: ASCII_CORNER,
        vertical_left: "|",
        vertical_right: "|",
        horizontal_top: "-",
        horizontal_bottom: "-",
    },
    junction: Junctions {
        tee_right: ASCII_CORNER,
        tee_left: ASCII_CORNER,
        tee_down: ASCII_CORNER,
        tee_up: ASCII_CORNER,
        cross: ASCII_CORNER,
    },
};

impl Glyphs {
    /// Cuadro del spinner para el número de tick dado; da la vuelta solo.
    pub fn spinner_frame(&self, tick: usize) -> &'static str {
        self.spinner[tick % SPINNER_FRAMES]
    }
}

#[cfg(test)]
mod tests {
    use ratatui::text::Span;

    use super::*;

    /// Todos los glifos de un juego, para recorrerlos en los tests.
    fn all_symbols(glyphs: &Glyphs) -> Vec<&'static str> {
        let border = glyphs.border;
        let junction = glyphs.junction;
        let mut symbols = vec![
            glyphs.play,
            glyphs.pause,
            glyphs.stop,
            glyphs.now_playing,
            glyphs.cursor,
            glyphs.directory,
            glyphs.playlist,
            glyphs.bar_filled,
            glyphs.bar_head,
            glyphs.bar_empty,
            glyphs.bar_separator,
            glyphs.bar_open,
            glyphs.bar_close,
            glyphs.bar_unknown,
            glyphs.bar_more,
            glyphs.meter_filled,
            glyphs.meter_empty,
            glyphs.flag_on,
            glyphs.flag_off,
            glyphs.flag_once,
            glyphs.flag_key_gap,
            glyphs.group_rule,
            glyphs.breadcrumb,
            glyphs.ellipsis,
            glyphs.scroll_up,
            glyphs.scroll_down,
            glyphs.scroll_track,
            glyphs.scroll_thumb,
            glyphs.connected,
            glyphs.disconnected,
            glyphs.spectrum_peak,
            glyphs.meter_peak,
            glyphs.scale_tick,
            glyphs.correlation_center,
            glyphs.correlation_marker,
            glyphs.page_active,
            glyphs.page_inactive,
            glyphs.message_info,
            glyphs.message_error,
            glyphs.input_cursor,
            border.top_left,
            border.top_right,
            border.bottom_left,
            border.bottom_right,
            border.vertical_left,
            border.vertical_right,
            border.horizontal_top,
            border.horizontal_bottom,
            junction.tee_right,
            junction.tee_left,
            junction.tee_down,
            junction.tee_up,
            junction.cross,
        ];
        symbols.extend(glyphs.spinner);
        for set in [
            glyphs.spectrum_levels,
            glyphs.mirror_up,
            glyphs.mirror_down,
            glyphs.shades,
            glyphs.meter_eighths,
        ] {
            symbols.extend(set);
        }
        if let Some(arrows) = glyphs.arrow_keys {
            symbols.extend([arrows.up, arrows.down, arrows.left, arrows.right]);
        }
        if let Plot::Cells(cells) = glyphs.plot {
            symbols.extend(cells.levels);
            symbols.extend(cells.density);
            symbols.extend([cells.join, cells.guide_left, cells.guide_right]);
        }
        symbols
    }

    #[test]
    fn ascii_set_is_pure_ascii() {
        for symbol in all_symbols(&ASCII) {
            assert!(symbol.is_ascii(), "glifo no ASCII: {symbol:?}");
        }
    }

    #[test]
    fn unicode_single_cell_glyphs_measure_one_column() {
        let multi_cell = [UNICODE.breadcrumb, UNICODE.bar_open, UNICODE.bar_close];
        let spacing = [UNICODE.flag_key_gap];
        for symbol in all_symbols(&UNICODE) {
            if multi_cell.contains(&symbol) || spacing.contains(&symbol) {
                continue;
            }
            assert_eq!(Span::raw(symbol).width(), 1, "glifo {symbol:?}");
        }
    }

    #[test]
    fn spectrum_levels_start_empty() {
        for glyphs in [UNICODE, ASCII] {
            assert_eq!(glyphs.spectrum_levels[0], " ");
        }
        assert_eq!(UNICODE.spectrum_levels.len(), 9);
        assert_eq!(ASCII.spectrum_levels.len(), 4);
    }

    #[test]
    fn level_sets_start_empty() {
        for glyphs in [UNICODE, ASCII] {
            for set in [glyphs.mirror_up, glyphs.mirror_down, glyphs.shades] {
                assert_eq!(set[0], " ");
            }
        }
        assert_eq!(UNICODE.meter_eighths.len(), 8);
        assert_eq!(UNICODE.meter_eighths.last(), Some(&UNICODE.scroll_thumb));
        assert_eq!(ASCII.shades.len(), 10);
    }

    #[test]
    fn braille_base_is_the_empty_braille_cell() {
        assert_eq!(char::from_u32(BRAILLE_BASE), Some('\u{2800}'));
        let full = char::from_u32(BRAILLE_BASE + u32::from(u8::MAX)).unwrap();
        assert_eq!(Span::raw(full.to_string()).width(), 1);
    }

    #[test]
    fn spinner_wraps_around() {
        assert_eq!(ASCII.spinner_frame(0), "|");
        assert_eq!(ASCII.spinner_frame(SPINNER_FRAMES + 1), "/");
    }
}
