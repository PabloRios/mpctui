//! Barras por banda (Barras y Espejo): cuántas entran, dónde va cada una, su altura en
//! sub-celdas, el color por altura y las etiquetas de frecuencia.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::domain::band_centers;
use crate::tui::views::chrome::Context;
use crate::tui::views::format::{frequency_tick, saturating_u16};
use crate::tui::views::theme;

pub const SPECTRUM_MIN_BANDS: usize = 8;
pub const SPECTRUM_MAX_BANDS: usize = 64;
/// Tercios de la altura, para el color.
const HEIGHT_ZONES: usize = 3;
/// Separación mínima entre dos etiquetas del eje.
const LABEL_GAP: u16 = 1;

/// Barras de 2 columnas con 1 de separación; en modo compacto, 1 columna sin separación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarShape {
    width: u16,
    gap: u16,
}

const FULL_BARS: BarShape = BarShape { width: 2, gap: 1 };
const COMPACT_BARS: BarShape = BarShape { width: 1, gap: 0 };

impl BarShape {
    pub fn for_context(context: &Context<'_>) -> Self {
        if context.is_compact() {
            COMPACT_BARS
        } else {
            FULL_BARS
        }
    }

    fn step(self) -> usize {
        usize::from(self.width + self.gap)
    }

    /// Cuántas bandas entran en `width` columnas, entre el mínimo y el máximo.
    pub fn bands_in(self, width: u16) -> usize {
        let bands = (usize::from(width) + usize::from(self.gap)) / self.step();
        bands.clamp(SPECTRUM_MIN_BANDS, SPECTRUM_MAX_BANDS)
    }

    fn total_width(self, bands: usize) -> usize {
        (bands * self.step()).saturating_sub(usize::from(self.gap))
    }
}

/// Las barras ubicadas en un área: cuántas entran y dónde empieza la primera, con el
/// sobrante repartido a los costados.
#[derive(Debug, Clone, Copy)]
pub struct BarLayout {
    pub shape: BarShape,
    /// Bandas que entran (el cuadro puede ser de otro ancho tras un cambio de tamaño,
    /// hasta que llegue el próximo).
    pub bands: usize,
    start: u16,
}

impl BarLayout {
    pub fn new(area: Rect, shape: BarShape, bands: usize) -> Self {
        let fitting = (usize::from(area.width) + usize::from(shape.gap)) / shape.step();
        let bands = bands.min(fitting);
        let spare = usize::from(area.width).saturating_sub(shape.total_width(bands));
        Self {
            shape,
            bands,
            start: area.x + saturating_u16(spare / 2),
        }
    }

    /// Primera columna de la barra `band`.
    pub fn x(&self, band: usize) -> u16 {
        self.start + saturating_u16(band * self.shape.step())
    }

    /// El texto de una barra: su glifo repetido a lo ancho.
    pub fn fill(&self, symbol: &str) -> String {
        symbol.repeat(usize::from(self.shape.width))
    }

    /// Etiquetas de frecuencia centradas debajo de la banda más cercana a cada una, sin
    /// salirse del área. Si una se pisa con la anterior, se omite. `pad` rodea cada
    /// etiqueta (el Espejo las corta dentro de una línea).
    pub fn axis_labels(&self, area: Rect, frequencies: &[u32], pad: &str) -> Vec<(u16, String)> {
        let centers = band_centers(self.bands);
        let mut labels = Vec::new();
        let mut free_from = area.x;
        if self.bands == 0 {
            return labels;
        }
        for hertz in frequencies {
            let label = format!("{pad}{}{pad}", frequency_tick(*hertz));
            let width = saturating_u16(Span::raw(&label).width());
            let center = self.x(nearest_band(&centers, *hertz as f32)) + self.shape.width / 2;
            let x = center
                .saturating_sub(width / 2)
                .min(area.right().saturating_sub(width));
            if x < free_from {
                continue;
            }
            free_from = x + width + LABEL_GAP;
            labels.push((x, label));
        }
        labels
    }
}

/// Altura de `level` en sub-celdas.
pub fn sub_cells(level: f32, rows: usize, steps: usize) -> usize {
    (level.clamp(0.0, 1.0) * (rows * steps) as f32).round() as usize
}

/// Glifo de la fila `row` (contando desde la base) de una barra de `rows` filas, con
/// los niveles `levels` (vacío primero); `None` si va vacía.
pub fn fill_glyph(
    level: f32,
    rows: usize,
    row: usize,
    levels: &'static [&'static str],
) -> Option<&'static str> {
    let steps = levels.len() - 1;
    let fill = sub_cells(level, rows, steps)
        .saturating_sub(row * steps)
        .min(steps);
    (fill > 0).then_some(levels[fill])
}

/// Verde el tercio de abajo, amarillo el del medio y rojo el de arriba.
pub fn zone_style(row: usize, rows: usize) -> Style {
    match (row * HEIGHT_ZONES) / rows.max(1) {
        0 => theme::SPECTRUM_LOW,
        1 => theme::SPECTRUM_MIDDLE,
        _ => theme::SPECTRUM_HIGH,
    }
}

/// La banda cuyo centro está más cerca de `hertz`, en escala logarítmica.
pub fn nearest_band(centers: &[f32], hertz: f32) -> usize {
    let distance = |center: &f32| (center.ln() - hertz.ln()).abs();
    centers
        .iter()
        .enumerate()
        .min_by(|first, second| distance(first.1).total_cmp(&distance(second.1)))
        .map(|(band, _)| band)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::band_edges;
    use crate::tui::glyphs::{ASCII, UNICODE};

    #[test]
    fn band_count_follows_the_width() {
        assert_eq!(FULL_BARS.bands_in(96), 32);
        assert_eq!(FULL_BARS.bands_in(10), SPECTRUM_MIN_BANDS);
        assert_eq!(FULL_BARS.bands_in(400), SPECTRUM_MAX_BANDS);
        assert_eq!(COMPACT_BARS.bands_in(68), SPECTRUM_MAX_BANDS);
        assert_eq!(COMPACT_BARS.bands_in(38), 38);
    }

    #[test]
    fn bars_are_centered() {
        let layout = BarLayout::new(Rect::new(0, 0, 12, 1), FULL_BARS, 3);
        assert_eq!((layout.x(0), layout.x(2)), (2, 8));
        let narrow = BarLayout::new(Rect::new(0, 0, 5, 1), FULL_BARS, 8);
        assert_eq!(narrow.bands, 2);
    }

    #[test]
    fn partial_cells_go_at_the_top() {
        // 2 filas × 8 niveles: 0,75 llena la de abajo y media de arriba.
        assert_eq!(fill_glyph(0.75, 2, 0, UNICODE.spectrum_levels), Some("█"));
        assert_eq!(fill_glyph(0.75, 2, 1, UNICODE.spectrum_levels), Some("▄"));
        assert_eq!(fill_glyph(0.75, 2, 1, ASCII.spectrum_levels), Some(":"));
        assert_eq!(fill_glyph(0.25, 2, 1, UNICODE.mirror_up), None);
        assert_eq!(fill_glyph(0.25, 2, 0, UNICODE.mirror_down), Some("▀"));
    }

    #[test]
    fn colors_go_by_thirds() {
        assert_eq!(zone_style(0, 9), theme::SPECTRUM_LOW);
        assert_eq!(zone_style(4, 9), theme::SPECTRUM_MIDDLE);
        assert_eq!(zone_style(8, 9), theme::SPECTRUM_HIGH);
    }

    #[test]
    fn labels_go_under_the_nearest_band_without_overlapping() {
        let centers = band_centers(32);
        let band = nearest_band(&centers, 1_000.0);
        let edges = band_edges(32);
        assert!(edges[band] <= 1_000.0 * 1.2 && 1_000.0 / 1.2 < edges[band + 1]);
        assert_eq!(nearest_band(&centers, 20_000.0), 31);
        let area = Rect::new(0, 0, 24, 1);
        let layout = BarLayout::new(area, FULL_BARS, 8);
        let labels = layout.axis_labels(area, &[63, 125, 250, 16_000], "");
        let texts: Vec<_> = labels.iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(texts.first(), Some(&"63"));
        assert_eq!(texts.last(), Some(&"16k"));
        assert!(labels.windows(2).all(|pair| pair[0].0 + 2 < pair[1].0));
    }
}
