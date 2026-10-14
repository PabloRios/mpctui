//! Los cuatro temas. Es el único lugar con colores, como `glyphs.rs` lo es de los
//! glifos: `terminal` usa los nombres ANSI (toma la paleta de la terminal) y los demás,
//! colores de 24 bits pensados para terminales oscuras.

use ratatui::style::Color;

use super::spec::{GradientSwatches, Rgb, RoleSwatches, Swatch, ThemeSpec};
use crate::config::ThemeId;

const fn hex(value: u32) -> Swatch {
    Swatch::Rgb(Rgb::hex(value))
}

const fn ansi(color: Color) -> Swatch {
    Swatch::Ansi(color)
}

const TEXT: Swatch = Swatch::TerminalText;

/// Fondo contra el que se mide el contraste de los temas de 24 bits.
#[cfg(test)]
pub const REFERENCE_BACKGROUND: Rgb = Rgb::hex(0x1e_1e_24);
/// Texto típico de una terminal oscura, para medirlo sobre la selección.
#[cfg(test)]
pub const REFERENCE_TEXT: Rgb = Rgb::hex(0xd8_d8_d8);

/// El tema por defecto y el que usan todos con 16 colores.
pub const TERMINAL: ThemeSpec = ThemeSpec {
    roles: RoleSwatches {
        text: TEXT,
        text_strong: TEXT,
        muted: ansi(Color::DarkGray),
        subtle: ansi(Color::DarkGray),
        accent: ansi(Color::Cyan),
        accent_alt: ansi(Color::Magenta),
        tab_active: ansi(Color::Cyan),
        playing: ansi(Color::Green),
        paused: ansi(Color::Yellow),
        artist: ansi(Color::LightBlue),
        album: ansi(Color::Magenta),
        duration: TEXT,
        directory: ansi(Color::LightBlue),
        playlist: ansi(Color::Magenta),
        match_text: ansi(Color::Yellow),
        selection: TEXT,
        success: ansi(Color::Green),
        warning: ansi(Color::Yellow),
        error: ansi(Color::LightRed),
        info: ansi(Color::Cyan),
        lossless: ansi(Color::Green),
        lossy: ansi(Color::Yellow),
        high_resolution: ansi(Color::Magenta),
        peak: ansi(Color::White),
    },
    gradients: GradientSwatches {
        progress: &[ansi(Color::Cyan)],
        volume: &[ansi(Color::Green), ansi(Color::Yellow), ansi(Color::Red)],
        bars: &[ansi(Color::Green), ansi(Color::Yellow), ansi(Color::Red)],
        heat: &[
            ansi(Color::DarkGray),
            ansi(Color::Blue),
            ansi(Color::Magenta),
            ansi(Color::Red),
            ansi(Color::Yellow),
        ],
        mirror_left: &[ansi(Color::Cyan)],
        mirror_right: &[ansi(Color::Magenta)],
        wave: &[ansi(Color::Cyan), ansi(Color::Magenta), ansi(Color::Red)],
    },
};

/// Frío: azul cielo, aguamarina y lavanda.
pub const MIDNIGHT: ThemeSpec = ThemeSpec {
    roles: RoleSwatches {
        text: TEXT,
        text_strong: TEXT,
        muted: hex(0x8a_93_a8),
        subtle: hex(0x3d_44_56),
        accent: hex(0x5f_b3_f9),
        accent_alt: hex(0xb4_8e_ff),
        tab_active: hex(0x5f_b3_f9),
        playing: hex(0x5e_e0_a0),
        paused: hex(0xf2_c4_6d),
        artist: hex(0x7f_d3_e6),
        album: hex(0xc9_a7_f5),
        duration: hex(0x9f_b4_d1),
        directory: hex(0x5f_b3_f9),
        playlist: hex(0xc9_a7_f5),
        match_text: hex(0xff_d1_66),
        selection: hex(0x25_2c_42),
        success: hex(0x5e_e0_a0),
        warning: hex(0xf2_c4_6d),
        error: hex(0xff_6b_81),
        info: hex(0x7f_d3_e6),
        lossless: hex(0x5e_e0_a0),
        lossy: hex(0xf2_c4_6d),
        high_resolution: hex(0xc9_a7_f5),
        peak: hex(0xe8_f1_ff),
    },
    gradients: GradientSwatches {
        progress: &[hex(0x5f_b3_f9), hex(0xb4_8e_ff)],
        volume: &[hex(0x5e_e0_a0), hex(0xf2_c4_6d), hex(0xff_6b_81)],
        bars: &[
            hex(0x2f_7d_e1),
            hex(0x5f_b3_f9),
            hex(0xb4_8e_ff),
            hex(0xf0_a6_ff),
        ],
        heat: &[
            hex(0x16_18_2e),
            hex(0x2f_3a_8f),
            hex(0x6b_4f_d6),
            hex(0x5f_b3_f9),
            hex(0xe8_f6_ff),
        ],
        mirror_left: &[hex(0x2f_7d_e1), hex(0x5f_b3_f9)],
        mirror_right: &[hex(0x7b_5b_d6), hex(0xc9_a7_f5)],
        wave: &[hex(0x5f_b3_f9), hex(0xb4_8e_ff), hex(0xff_6b_81)],
    },
};

/// Cálido: ámbar, naranja y el verde de los vúmetros de un equipo hi-fi.
pub const VINYL: ThemeSpec = ThemeSpec {
    roles: RoleSwatches {
        text: TEXT,
        text_strong: TEXT,
        muted: hex(0xa8_98_7f),
        subtle: hex(0x4a_40_38),
        accent: hex(0xff_b4_54),
        accent_alt: hex(0xff_7a_45),
        tab_active: hex(0xff_b4_54),
        playing: hex(0xb8_cc_52),
        paused: hex(0xff_d5_80),
        artist: hex(0xe8_b9_8a),
        album: hex(0xf2_9e_74),
        duration: hex(0xb8_a5_8c),
        directory: hex(0xff_b4_54),
        playlist: hex(0xf2_9e_74),
        match_text: hex(0xff_d5_80),
        selection: hex(0x3a_2e_24),
        success: hex(0xb8_cc_52),
        warning: hex(0xff_d5_80),
        error: hex(0xf0_54_4c),
        info: hex(0x8e_c0_a8),
        lossless: hex(0xb8_cc_52),
        lossy: hex(0xff_b4_54),
        high_resolution: hex(0xff_8f_b1),
        peak: hex(0xff_f1_d6),
    },
    gradients: GradientSwatches {
        progress: &[hex(0xff_b4_54), hex(0xff_7a_45)],
        volume: &[hex(0xb8_cc_52), hex(0xff_b4_54), hex(0xf0_54_4c)],
        bars: &[
            hex(0x8a_9a_3a),
            hex(0xb8_cc_52),
            hex(0xff_b4_54),
            hex(0xf0_54_4c),
        ],
        heat: &[
            hex(0x1a_14_10),
            hex(0x5a_1f_1a),
            hex(0xc2_42_1f),
            hex(0xff_b4_54),
            hex(0xff_f1_c4),
        ],
        mirror_left: &[hex(0xc9_8a_2e), hex(0xff_b4_54)],
        mirror_right: &[hex(0xc0_51_2b), hex(0xff_7a_45)],
        wave: &[hex(0xff_b4_54), hex(0xff_7a_45), hex(0xf0_54_4c)],
    },
};

/// Synthwave: magenta y cian eléctricos sobre violeta.
pub const NEON: ThemeSpec = ThemeSpec {
    roles: RoleSwatches {
        text: TEXT,
        text_strong: TEXT,
        muted: hex(0x9a_90_c2),
        subtle: hex(0x3d_33_5c),
        accent: hex(0xff_4f_d8),
        accent_alt: hex(0x29_e7_ff),
        tab_active: hex(0xff_4f_d8),
        playing: hex(0x29_e7_ff),
        paused: hex(0xff_e1_56),
        artist: hex(0xa9_9b_ff),
        album: hex(0xff_8f_d8),
        duration: hex(0xb3_a8_e0),
        directory: hex(0x29_e7_ff),
        playlist: hex(0xff_4f_d8),
        match_text: hex(0xff_e1_56),
        selection: hex(0x3a_1f_4d),
        success: hex(0x3d_ff_a8),
        warning: hex(0xff_e1_56),
        error: hex(0xff_3b_6b),
        info: hex(0x29_e7_ff),
        lossless: hex(0x3d_ff_a8),
        lossy: hex(0xff_e1_56),
        high_resolution: hex(0xff_4f_d8),
        peak: hex(0xff_ff_ff),
    },
    gradients: GradientSwatches {
        progress: &[hex(0xff_4f_d8), hex(0x29_e7_ff)],
        volume: &[hex(0x3d_ff_a8), hex(0xff_e1_56), hex(0xff_3b_6b)],
        bars: &[
            hex(0x6a_2c_ff),
            hex(0xff_4f_d8),
            hex(0xff_9a_d5),
            hex(0x29_e7_ff),
        ],
        heat: &[
            hex(0x0d_02_21),
            hex(0x3b_0f_70),
            hex(0xb1_20_7c),
            hex(0xff_4f_d8),
            hex(0x29_e7_ff),
        ],
        mirror_left: &[hex(0xb1_20_7c), hex(0xff_4f_d8)],
        mirror_right: &[hex(0x0f_8f_b0), hex(0x29_e7_ff)],
        wave: &[hex(0x29_e7_ff), hex(0xff_4f_d8), hex(0xff_3b_6b)],
    },
};

pub fn spec(theme: ThemeId) -> &'static ThemeSpec {
    match theme {
        ThemeId::Terminal => &TERMINAL,
        ThemeId::Midnight => &MIDNIGHT,
        ThemeId::Vinyl => &VINYL,
        ThemeId::Neon => &NEON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::theme::roles::{GradientRole, Role};

    /// Un degradado tiene de una a cinco paradas.
    const MAX_STOPS: usize = 5;

    /// Contraste mínimo de WCAG para el texto y para los gráficos.
    const TEXT_CONTRAST: f64 = 4.5;
    const GRAPHIC_CONTRAST: f64 = 3.0;
    const TRUECOLOR_THEMES: [ThemeId; 3] = [ThemeId::Midnight, ThemeId::Vinyl, ThemeId::Neon];

    /// Luminancia relativa de WCAG 2.
    fn luminance(color: Rgb) -> f64 {
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
    }

    fn contrast(first: Rgb, second: Rgb) -> f64 {
        let (first, second) = (luminance(first), luminance(second));
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    fn rgb(swatch: Swatch) -> Option<Rgb> {
        match swatch {
            Swatch::Rgb(color) => Some(color),
            Swatch::Ansi(_) | Swatch::TerminalText => None,
        }
    }

    /// Lo que pide cada rol: el texto 4,5:1, los gráficos 3:1. `subtle` es decorativo
    /// (bordes y partes vacías: se aparta a propósito y nada depende solo de él), y la
    /// selección es un fondo, que se mide aparte.
    fn required_contrast(role: Role) -> Option<f64> {
        match role {
            Role::Subtle | Role::Selection | Role::Text | Role::TextStrong => None,
            Role::Peak => Some(GRAPHIC_CONTRAST),
            _ => Some(TEXT_CONTRAST),
        }
    }

    #[test]
    fn text_and_graphics_contrast_with_the_reference_background() {
        for theme in TRUECOLOR_THEMES {
            for role in Role::ALL {
                let Some(required) = required_contrast(role) else {
                    continue;
                };
                let color = rgb(spec(theme).roles.get(role)).expect("color de 24 bits");
                let ratio = contrast(color, REFERENCE_BACKGROUND);
                assert!(ratio >= required, "{theme:?} {role:?}: {ratio:.2}");
            }
        }
    }

    #[test]
    fn text_stays_readable_on_the_selection() {
        for theme in TRUECOLOR_THEMES {
            let roles = &spec(theme).roles;
            let selection = rgb(roles.selection).unwrap();
            assert!(contrast(REFERENCE_TEXT, selection) >= TEXT_CONTRAST);
            let muted = rgb(roles.muted).unwrap();
            assert!(contrast(muted, selection) >= GRAPHIC_CONTRAST, "{theme:?}");
        }
    }

    #[test]
    fn the_terminal_theme_only_uses_named_colors() {
        for role in Role::ALL {
            assert!(rgb(TERMINAL.roles.get(role)).is_none(), "{role:?}");
        }
        for role in GradientRole::ALL {
            assert!(
                TERMINAL
                    .gradients
                    .get(role)
                    .iter()
                    .all(|stop| rgb(*stop).is_none())
            );
        }
    }

    #[test]
    fn every_gradient_has_one_to_five_stops() {
        for theme in ThemeId::ALL {
            for role in GradientRole::ALL {
                let stops = spec(theme).gradients.get(role).len();
                assert!((1..=MAX_STOPS).contains(&stops), "{theme:?} {role:?}");
            }
        }
    }
}
