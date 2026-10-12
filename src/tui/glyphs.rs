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
    pub message_info: &'static str,
    pub message_error: &'static str,
    /// Cursor al final del texto que se está escribiendo.
    pub input_cursor: &'static str,
    /// Nombres de las flechas del teclado en la ayuda.
    pub key_up: &'static str,
    pub key_down: &'static str,
    pub key_left: &'static str,
    pub key_right: &'static str,
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
    message_info: "·",
    message_error: "✖",
    input_cursor: "_",
    key_up: "↑",
    key_down: "↓",
    key_left: "←",
    key_right: "→",
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
    message_info: "(i)",
    message_error: "(x)",
    input_cursor: "_",
    key_up: "Arriba",
    key_down: "Abajo",
    key_left: "Izq",
    key_right: "Der",
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
            glyphs.message_info,
            glyphs.message_error,
            glyphs.input_cursor,
            glyphs.key_up,
            glyphs.key_down,
            glyphs.key_left,
            glyphs.key_right,
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
        symbols.extend(glyphs.spectrum_levels);
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
    fn spinner_wraps_around() {
        assert_eq!(ASCII.spinner_frame(0), "|");
        assert_eq!(ASCII.spinner_frame(SPINNER_FRAMES + 1), "/");
    }
}
