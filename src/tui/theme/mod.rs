//! Colores de la interfaz. Las vistas piden un `Role` (o un `GradientRole` con una
//! posición) y el tema resuelto devuelve el estilo; ninguna vista elige colores, tema
//! ni profundidad. El mismo tema se ve lo mejor posible en cada terminal: colores de 24
//! bits, el índice más cercano de 256 o, con 16 colores, el tema `terminal`. Sin color
//! quedan los modificadores y los glifos.

mod gradient;
mod palettes;
mod roles;
mod spec;
mod xterm256;

use ratatui::style::{Color, Modifier, Style};

pub use self::gradient::Gradient;
pub use self::roles::{GradientRole, Role};
use self::spec::Swatch;
use crate::config::{ColorDepth, ThemeId};

/// Líneas de la letra después de la actual en las que el texto se apaga hasta `muted`.
pub const LYRICS_FADE_LINES: usize = 6;
/// Con 16 colores no hay mezcla: estas líneas siguen como texto y el resto, `muted`.
const PLAIN_FADE_LINES: usize = 2;

/// Un tema listo para dibujar: un estilo por rol y la tabla de cada degradado.
#[derive(Debug, Clone)]
pub struct Theme {
    id: ThemeId,
    depth: ColorDepth,
    styles: [Style; Role::COUNT],
    gradients: [Gradient; GradientRole::COUNT],
    /// Desvanecido de la letra, de `peak` a `muted`: `text` es el color de la terminal y
    /// no se puede mezclar. Sin colores de tema, `None` (va por tramos).
    lyrics_fade: Option<Gradient>,
}

impl Theme {
    /// Con 16 colores o sin color se usa siempre `terminal`: los colores son los de la
    /// paleta de la terminal, no los del tema.
    pub fn resolve(id: ThemeId, depth: ColorDepth) -> Self {
        let spec = if depth.shows_themes() {
            palettes::spec(id)
        } else {
            &palettes::TERMINAL
        };
        let fade_stops = [spec.roles.peak, spec.roles.muted];
        let mixable = fade_stops.iter().all(|stop| matches!(stop, Swatch::Rgb(_)));
        Self {
            id,
            depth,
            styles: Role::ALL.map(|role| role_style(role, spec.roles.get(role), depth)),
            gradients: GradientRole::ALL
                .map(|role| Gradient::resolve(spec.gradients.get(role), depth)),
            lyrics_fade: (mixable && depth.shows_themes())
                .then(|| Gradient::resolve(&fade_stops, depth)),
        }
    }

    /// El tema pedido (con 16 colores se dibuja con `terminal`).
    pub fn id(&self) -> ThemeId {
        self.id
    }

    pub fn depth(&self) -> ColorDepth {
        self.depth
    }

    pub fn style(&self, role: Role) -> Style {
        self.styles[role.index()]
    }

    /// El rol en negrita.
    pub fn strong(&self, role: Role) -> Style {
        self.style(role).add_modifier(Modifier::BOLD)
    }

    /// El estilo del degradado en `position`, entre 0 y 1.
    pub fn gradient(&self, role: GradientRole, position: f32) -> Style {
        self.gradients[role.index()].at(position)
    }

    /// Una línea de la letra a `distance` líneas después de la actual: se apaga hasta
    /// `muted` en `LYRICS_FADE_LINES` líneas.
    pub fn faded(&self, distance: usize) -> Style {
        if distance >= LYRICS_FADE_LINES {
            return self.style(Role::Muted);
        }
        match &self.lyrics_fade {
            Some(fade) => {
                let position = distance.saturating_sub(1) as f32 / (LYRICS_FADE_LINES - 1) as f32;
                fade.at(position)
            }
            None if distance <= PLAIN_FADE_LINES => self.style(Role::Text),
            None => self.style(Role::Muted),
        }
    }
}

/// La selección es un fondo con los colores del tema; si no, video inverso.
fn role_style(role: Role, swatch: Swatch, depth: ColorDepth) -> Style {
    let color = paint(swatch, depth);
    if role == Role::Selection {
        return match (swatch, color) {
            (Swatch::Rgb(_), Some(color)) => Style::new().bg(color),
            _ => Style::new().add_modifier(Modifier::REVERSED),
        };
    }
    painted(color).add_modifier(role.modifier())
}

/// El color de la terminal para un color de tema; `None` deja el de la terminal.
fn paint(swatch: Swatch, depth: ColorDepth) -> Option<Color> {
    match (swatch, depth) {
        (_, ColorDepth::NoColor) | (Swatch::TerminalText, _) => None,
        (Swatch::Ansi(color), _) => Some(color),
        (Swatch::Rgb(color), ColorDepth::TrueColor) => {
            Some(Color::Rgb(color.red, color.green, color.blue))
        }
        (Swatch::Rgb(color), ColorDepth::Palette256) => {
            Some(Color::Indexed(xterm256::nearest_index(color)))
        }
        // No pasa: con 16 colores rige `terminal`, que solo usa colores con nombre.
        (Swatch::Rgb(_), ColorDepth::Ansi16) => None,
    }
}

/// Un estilo con ese color de frente, si hay.
fn painted(color: Option<Color>) -> Style {
    match color {
        Some(color) => Style::new().fg(color),
        None => Style::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_colors_always_use_the_terminal_theme() {
        let neon = Theme::resolve(ThemeId::Neon, ColorDepth::Ansi16);
        let terminal = Theme::resolve(ThemeId::Terminal, ColorDepth::Ansi16);
        assert_eq!(neon.styles, terminal.styles);
        assert_eq!(neon.id(), ThemeId::Neon);
        assert_eq!(neon.style(Role::Accent).fg, Some(Color::Cyan));
    }

    #[test]
    fn truecolor_themes_paint_their_colors() {
        let midnight = Theme::resolve(ThemeId::Midnight, ColorDepth::TrueColor);
        assert_eq!(
            midnight.style(Role::Accent).fg,
            Some(Color::Rgb(0x5f, 0xb3, 0xf9))
        );
        assert_eq!(
            midnight.style(Role::Selection),
            Style::new().bg(Color::Rgb(0x25, 0x2c, 0x42))
        );
        let indexed = Theme::resolve(ThemeId::Midnight, ColorDepth::Palette256);
        assert!(matches!(
            indexed.style(Role::Accent).fg,
            Some(Color::Indexed(_))
        ));
    }

    #[test]
    fn without_color_only_modifiers_remain() {
        let theme = Theme::resolve(ThemeId::Vinyl, ColorDepth::NoColor);
        for role in Role::ALL {
            let style = theme.style(role);
            assert_eq!((style.fg, style.bg), (None, None), "{role:?}");
        }
        assert_eq!(
            theme.style(Role::Selection),
            Style::new().add_modifier(Modifier::REVERSED)
        );
        assert!(
            theme
                .style(Role::TabActive)
                .add_modifier
                .contains(Modifier::BOLD | Modifier::REVERSED)
        );
        assert_eq!(theme.gradient(GradientRole::Bars, 0.9), Style::new());
    }

    #[test]
    fn lyrics_fade_towards_muted() {
        let plain = Theme::resolve(ThemeId::Terminal, ColorDepth::Ansi16);
        assert_eq!(plain.faded(1), plain.style(Role::Text));
        assert_eq!(plain.faded(PLAIN_FADE_LINES), plain.style(Role::Text));
        assert_eq!(plain.faded(PLAIN_FADE_LINES + 1), plain.style(Role::Muted));
        let vinyl = Theme::resolve(ThemeId::Vinyl, ColorDepth::TrueColor);
        assert_eq!(vinyl.faded(1).fg, vinyl.style(Role::Peak).fg);
        assert_eq!(vinyl.faded(LYRICS_FADE_LINES), vinyl.style(Role::Muted));
        let middle = vinyl.faded(LYRICS_FADE_LINES / 2).fg;
        assert_ne!(middle, vinyl.faded(1).fg);
        assert_ne!(middle, vinyl.style(Role::Muted).fg);
    }

    #[test]
    fn the_terminal_selection_is_reverse_video() {
        let theme = Theme::resolve(ThemeId::Terminal, ColorDepth::TrueColor);
        assert_eq!(
            theme.style(Role::Selection),
            Style::new().add_modifier(Modifier::REVERSED)
        );
        assert_eq!(theme.style(Role::Text), Style::new());
    }
}
