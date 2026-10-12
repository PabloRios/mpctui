//! Barras: una por banda, alineadas abajo, con picos y el eje de frecuencias debajo.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;

use super::columns::{BarLayout, BarShape, fill_glyph, sub_cells, zone_style};
use crate::domain::{BandFrame, SPECTRUM_MAX_HZ, SPECTRUM_MIN_HZ, VisualRequest, VisualizerStyle};
use crate::tui::glyphs::Glyphs;
use crate::tui::views::chrome::Context;
use crate::tui::views::format::{frequency, saturating_u16};
use crate::tui::views::theme;

const AXIS_ROWS: u16 = 1;
/// Frecuencias del eje, en Hz.
const AXIS_FREQUENCIES: [u32; 9] = [63, 125, 250, 500, 1_000, 2_000, 4_000, 8_000, 16_000];

pub fn request(area: Rect, context: &Context<'_>) -> VisualRequest {
    VisualRequest {
        style: VisualizerStyle::Bars,
        columns: BarShape::for_context(context).bands_in(area.width),
        rows: 0,
    }
}

/// `32 bandas  40 Hz - 16 kHz`.
pub fn details(bands: &BandFrame) -> String {
    format!(
        "{} bandas  {} - {}",
        bands.levels.len(),
        frequency(SPECTRUM_MIN_HZ),
        frequency(SPECTRUM_MAX_HZ)
    )
}

/// En modo compacto, sin eje.
pub fn render(frame: &mut Frame, area: Rect, bands: &BandFrame, context: &Context<'_>) {
    let axis_rows = if context.is_compact() { 0 } else { AXIS_ROWS };
    let [bars_area, axis_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(axis_rows)]).areas(area);
    let layout = BarLayout::new(
        bars_area,
        BarShape::for_context(context),
        bands.levels.len(),
    );
    let rows = usize::from(bars_area.height);
    for band in 0..layout.bands {
        let column = BarColumn {
            level: bands.levels[band],
            peak: bands.peaks.get(band).copied().unwrap_or_default(),
            rows,
        };
        for row in 0..rows {
            let Some((symbol, style)) = column.cell(row, context.glyphs) else {
                continue;
            };
            let y = bars_area.bottom() - 1 - saturating_u16(row);
            frame
                .buffer_mut()
                .set_string(layout.x(band), y, layout.fill(symbol), style);
        }
    }
    if !axis_area.is_empty() {
        for (x, label) in layout.axis_labels(axis_area, &AXIS_FREQUENCIES, "") {
            frame
                .buffer_mut()
                .set_string(x, axis_area.y, label, theme::MUTED);
        }
    }
}

/// Una barra en sub-celdas, con la marca de su pico.
struct BarColumn {
    level: f32,
    peak: f32,
    rows: usize,
}

impl BarColumn {
    /// Glifo y estilo de la fila `row` contando desde abajo; `None` si va vacía. El
    /// pico solo se dibuja en una celda vacía.
    fn cell(&self, row: usize, glyphs: &Glyphs) -> Option<(&'static str, Style)> {
        let style = zone_style(row, self.rows);
        if let Some(symbol) = fill_glyph(self.level, self.rows, row, glyphs.spectrum_levels) {
            return Some((symbol, style));
        }
        let steps = glyphs.spectrum_levels.len() - 1;
        let peak = sub_cells(self.peak, self.rows, steps);
        (peak > 0 && peak / steps == row).then_some((glyphs.spectrum_peak, style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::{ASCII, UNICODE};

    #[test]
    fn the_peak_sits_above_a_full_bar() {
        let half = BarColumn {
            level: 0.5,
            peak: 0.5,
            rows: 2,
        };
        assert_eq!(half.cell(0, &UNICODE).unwrap().0, "█");
        assert_eq!(half.cell(1, &UNICODE).unwrap().0, "▔");
    }

    #[test]
    fn peaks_only_show_on_empty_cells() {
        let column = BarColumn {
            level: 0.2,
            peak: 0.9,
            rows: 4,
        };
        let cells: Vec<_> = (0..4)
            .map(|row| column.cell(row, &ASCII).map(|(symbol, _)| symbol))
            .collect();
        assert_eq!(cells, [Some(":"), None, None, Some("-")]);
        let empty = BarColumn {
            level: 0.0,
            peak: 0.0,
            rows: 4,
        };
        assert!((0..4).all(|row| empty.cell(row, &ASCII).is_none()));
    }
}
