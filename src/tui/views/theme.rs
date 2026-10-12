//! Paleta y estilos con nombre; ninguna vista usa colores sueltos. Solo los 16 colores
//! ANSI básicos: todo estado se distingue además por un glifo o un modificador.

use ratatui::style::{Color, Modifier, Style};

pub const PLAIN: Style = Style::new();
pub const ACCENT: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
pub const MUTED: Style = Style::new().fg(Color::DarkGray);
pub const BOLD: Style = Style::new().add_modifier(Modifier::BOLD);
pub const ERROR: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
pub const INFO: Style = Style::new().fg(Color::Green);
pub const SELECTED_ROW: Style = Style::new().add_modifier(Modifier::REVERSED);
pub const CURRENT_SONG: Style = ACCENT;
/// Pestaña o columna con foco.
pub const FOCUSED: Style = Style::new()
    .add_modifier(Modifier::BOLD)
    .add_modifier(Modifier::REVERSED);
pub const TABLE_HEADER: Style = MUTED;
pub const BORDER: Style = Style::new().fg(Color::DarkGray);
pub const BAR_FILLED: Style = Style::new().fg(Color::Cyan);
pub const BAR_HEAD: Style = ACCENT;
pub const BAR_EMPTY: Style = Style::new().fg(Color::DarkGray);
pub const BAR_SEPARATOR: Style = BOLD;
pub const GROUP_TITLE: Style = BOLD;
pub const GROUP_RULE: Style = MUTED;
pub const MODE_ON: Style = Style::new().fg(Color::Cyan);
pub const MODE_OFF: Style = Style::new().add_modifier(Modifier::DIM);
/// Tercios de altura del espectro: el color solo refuerza lo que ya dice la altura.
pub const SPECTRUM_LOW: Style = Style::new().fg(Color::Green);
pub const SPECTRUM_MIDDLE: Style = Style::new().fg(Color::Yellow);
pub const SPECTRUM_HIGH: Style = Style::new().fg(Color::Red);
/// Puntos de la Onda y del Vectorscopio.
pub const SCOPE: Style = Style::new().fg(Color::Cyan);
/// Guías del Vectorscopio: quedan detrás de la señal.
pub const SCOPE_GUIDE: Style = MUTED;
/// Intensidad de la Cascada, de baja a alta: el sombreado ya la transmite.
pub const WATERFALL_SHADES: [Style; 4] = [
    Style::new().fg(Color::DarkGray),
    Style::new().fg(Color::Cyan),
    Style::new().fg(Color::Yellow),
    Style::new().fg(Color::Red),
];
/// Texto que coincide con la búsqueda.
pub const MATCH: Style = Style::new()
    .add_modifier(Modifier::BOLD)
    .add_modifier(Modifier::UNDERLINED);
