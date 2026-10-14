//! Degradados. Con colores de tema se interpola en OKLab (en RGB, la mezcla de azul y
//! amarillo pasa por gris) y se precalcula una tabla; con colores ANSI van escalonados:
//! un color por tramo. El visualizador pide cientos de colores por cuadro: `at` solo
//! indexa.

use ratatui::style::Style;

use super::spec::{Rgb, Swatch};
use super::{paint, painted};
use crate::config::ColorDepth;

/// Colores de la tabla precalculada de un degradado interpolado.
pub const GRADIENT_STEPS: usize = 64;

/// Un degradado ya resuelto para una profundidad de color.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    steps: Vec<Style>,
    /// Interpolado: `t` elige el paso más cercano. Escalonado: `t` elige el tramo.
    interpolated: bool,
}

impl Gradient {
    /// Interpola si todas las paradas son de 24 bits y la terminal muestra temas; si no,
    /// escalonado. Sin color, un solo estilo vacío.
    pub fn resolve(stops: &[Swatch], depth: ColorDepth) -> Self {
        if depth == ColorDepth::NoColor {
            return Self {
                steps: vec![Style::new()],
                interpolated: false,
            };
        }
        let colors: Option<Vec<Rgb>> = stops.iter().map(|stop| rgb(*stop)).collect();
        match colors {
            Some(colors) if depth.shows_themes() && !colors.is_empty() => Self {
                steps: (0..GRADIENT_STEPS)
                    .map(|step| step as f32 / (GRADIENT_STEPS - 1) as f32)
                    .map(|position| interpolate(&colors, position))
                    .map(|color| painted(paint(Swatch::Rgb(color), depth)))
                    .collect(),
                interpolated: true,
            },
            _ => Self {
                steps: stops
                    .iter()
                    .map(|stop| painted(paint(*stop, depth)))
                    .collect(),
                interpolated: false,
            },
        }
    }

    /// El estilo en `position`, entre 0 y 1 (fuera de rango se recorta).
    pub fn at(&self, position: f32) -> Style {
        let Some(last) = self.steps.len().checked_sub(1) else {
            return Style::new();
        };
        let position = if position.is_finite() {
            position.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let index = if self.interpolated {
            (position * last as f32).round() as usize
        } else {
            (position * self.steps.len() as f32) as usize
        };
        self.steps[index.min(last)]
    }
}

fn rgb(swatch: Swatch) -> Option<Rgb> {
    match swatch {
        Swatch::Rgb(color) => Some(color),
        Swatch::Ansi(_) | Swatch::TerminalText => None,
    }
}

/// Un color en OKLab: luminosidad y los dos ejes de tono.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Oklab {
    lightness: f32,
    green_red: f32,
    blue_yellow: f32,
}

/// El color en `position` entre paradas repartidas de forma pareja.
pub fn interpolate(stops: &[Rgb], position: f32) -> Rgb {
    let Some(last) = stops.len().checked_sub(1) else {
        return Rgb::hex(0);
    };
    if last == 0 {
        return stops[0];
    }
    let scaled = position.clamp(0.0, 1.0) * last as f32;
    let segment = (scaled.floor() as usize).min(last - 1);
    let fraction = scaled - segment as f32;
    let start = to_oklab(stops[segment]);
    let end = to_oklab(stops[segment + 1]);
    let mix = |from: f32, to: f32| from + (to - from) * fraction;
    from_oklab(Oklab {
        lightness: mix(start.lightness, end.lightness),
        green_red: mix(start.green_red, end.green_red),
        blue_yellow: mix(start.blue_yellow, end.blue_yellow),
    })
}

/// Componente sRGB (0–255) a lineal (0–1).
fn to_linear(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Lineal (0–1) a componente sRGB (0–255).
fn from_linear(value: f32) -> u8 {
    let encoded = if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Conversión de Björn Ottosson (2020).
fn to_oklab(color: Rgb) -> Oklab {
    let (red, green, blue) = (
        to_linear(color.red),
        to_linear(color.green),
        to_linear(color.blue),
    );
    let long = (0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue).cbrt();
    let medium = (0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue).cbrt();
    let short = (0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue).cbrt();
    Oklab {
        lightness: 0.210_454_26 * long + 0.793_617_8 * medium - 0.004_072_047 * short,
        green_red: 1.977_998_5 * long - 2.428_592_2 * medium + 0.450_593_7 * short,
        blue_yellow: 0.025_904_037 * long + 0.782_771_77 * medium - 0.808_675_77 * short,
    }
}

fn from_oklab(color: Oklab) -> Rgb {
    let Oklab {
        lightness,
        green_red,
        blue_yellow,
    } = color;
    let long = (lightness + 0.396_337_78 * green_red + 0.215_803_76 * blue_yellow).powi(3);
    let medium = (lightness - 0.105_561_346 * green_red - 0.063_854_17 * blue_yellow).powi(3);
    let short = (lightness - 0.089_484_18 * green_red - 1.291_485_5 * blue_yellow).powi(3);
    Rgb {
        red: from_linear(4.076_741_7 * long - 3.307_711_6 * medium + 0.230_969_94 * short),
        green: from_linear(-1.268_438 * long + 2.609_757_4 * medium - 0.341_319_38 * short),
        blue: from_linear(-0.004_196_086_3 * long - 0.703_418_6 * medium + 1.707_614_7 * short),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    const BLACK: Rgb = Rgb::hex(0x00_00_00);
    const WHITE: Rgb = Rgb::hex(0xff_ff_ff);

    fn foreground(style: Style) -> Option<Color> {
        style.fg
    }

    #[test]
    fn oklab_round_trips_and_mixes_by_perceived_lightness() {
        let sky = Rgb::hex(0x5f_b3_f9);
        assert_eq!(from_oklab(to_oklab(sky)), sky);
        // Calculados a mano con las fórmulas de Ottosson: el gris medio de OKLab es
        // más oscuro que el de RGB (127) y el violeta no pasa por gris.
        assert_eq!(interpolate(&[BLACK, WHITE], 0.5), Rgb::hex(0x63_63_63));
        let red_blue = [Rgb::hex(0xff_00_00), Rgb::hex(0x00_00_ff)];
        assert_eq!(interpolate(&red_blue, 0.5), Rgb::hex(0x8c_53_a2));
    }

    #[test]
    fn the_ends_are_the_extreme_stops() {
        let stops = [
            Swatch::Rgb(BLACK),
            Swatch::Rgb(Rgb::hex(0x5f_b3_f9)),
            Swatch::Rgb(WHITE),
        ];
        let gradient = Gradient::resolve(&stops, ColorDepth::TrueColor);
        assert_eq!(foreground(gradient.at(0.0)), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(
            foreground(gradient.at(1.0)),
            Some(Color::Rgb(255, 255, 255))
        );
        assert_eq!(gradient.at(-3.0), gradient.at(0.0));
        assert_eq!(gradient.at(f32::NAN), gradient.at(0.0));
    }

    #[test]
    fn the_table_matches_direct_interpolation() {
        let colors = [
            Rgb::hex(0x2f_7d_e1),
            Rgb::hex(0xb4_8e_ff),
            Rgb::hex(0xff_6b_81),
        ];
        let stops = colors.map(Swatch::Rgb);
        let gradient = Gradient::resolve(&stops, ColorDepth::TrueColor);
        for step in 0..GRADIENT_STEPS {
            let position = step as f32 / (GRADIENT_STEPS - 1) as f32;
            let expected = interpolate(&colors, position);
            let color = Color::Rgb(expected.red, expected.green, expected.blue);
            assert_eq!(foreground(gradient.at(position)), Some(color), "{step}");
        }
    }

    #[test]
    fn named_colors_go_in_steps() {
        let stops = [Color::Green, Color::Yellow, Color::Red].map(Swatch::Ansi);
        let gradient = Gradient::resolve(&stops, ColorDepth::TrueColor);
        assert_eq!(foreground(gradient.at(0.0)), Some(Color::Green));
        assert_eq!(foreground(gradient.at(0.32)), Some(Color::Green));
        assert_eq!(foreground(gradient.at(0.5)), Some(Color::Yellow));
        assert_eq!(foreground(gradient.at(0.7)), Some(Color::Red));
        assert_eq!(foreground(gradient.at(1.0)), Some(Color::Red));
        for step in 0..=100 {
            let color = foreground(gradient.at(step as f32 / 100.0));
            assert!(
                [Color::Green, Color::Yellow, Color::Red]
                    .map(Some)
                    .contains(&color),
                "{color:?}"
            );
        }
    }

    #[test]
    fn indexed_colors_are_quantized_and_no_color_is_empty() {
        let stops = [Swatch::Rgb(BLACK), Swatch::Rgb(WHITE)];
        let indexed = Gradient::resolve(&stops, ColorDepth::Palette256);
        assert!(matches!(
            foreground(indexed.at(0.5)),
            Some(Color::Indexed(_))
        ));
        let plain = Gradient::resolve(&stops, ColorDepth::NoColor);
        assert_eq!(plain.at(0.7), Style::new());
    }
}
