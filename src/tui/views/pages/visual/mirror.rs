//! Espejo estéreo: el canal L crece hacia arriba y el R hacia abajo desde un eje central
//! con las frecuencias dentro de la línea. Resolución de media celda en las dos mitades.

use ratatui::Frame;
use ratatui::layout::Rect;

use super::columns::{BarLayout, BarShape, fill_glyph, zone_style};
use crate::domain::{BandFrame, VisualRequest, VisualizerStyle};
use crate::tui::views::chrome::Context;
use crate::tui::views::format::saturating_u16;
use crate::tui::views::theme;

/// Columnas a la izquierda para las letras `L` y `R`.
const LABEL_MARGIN: u16 = 3;
const AXIS_ROWS: u16 = 1;
/// Frecuencias del eje central, en Hz.
const AXIS_FREQUENCIES: [u32; 5] = [63, 250, 1_000, 4_000, 16_000];
/// Espacio a cada lado de una etiqueta, para cortar la línea del eje.
const LABEL_PAD: &str = " ";
const LEFT_LABEL: &str = "L";
const RIGHT_LABEL: &str = "R";

/// Área de las barras: sin el margen de las letras en modo completo.
fn bars_area(area: Rect, context: &Context<'_>) -> Rect {
    if context.is_compact() {
        return area;
    }
    let margin = LABEL_MARGIN.min(area.width);
    Rect {
        x: area.x + margin,
        width: area.width - margin,
        ..area
    }
}

pub fn request(area: Rect, context: &Context<'_>) -> VisualRequest {
    VisualRequest {
        style: VisualizerStyle::Mirror,
        columns: BarShape::for_context(context).bands_in(bars_area(area, context).width),
        rows: 0,
    }
}

/// `31 bandas por canal  L arriba, R abajo`.
pub fn details(left: &BandFrame) -> String {
    format!(
        "{} bandas por canal  {LEFT_LABEL} arriba, {RIGHT_LABEL} abajo",
        left.levels.len()
    )
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    left: &BandFrame,
    right: &BandFrame,
    context: &Context<'_>,
) {
    let bars = bars_area(area, context);
    let half = area.height.saturating_sub(AXIS_ROWS) / 2;
    if half == 0 {
        return;
    }
    let axis_y = area.y + half;
    let layout = BarLayout::new(bars, BarShape::for_context(context), left.levels.len());
    let rows = usize::from(half);
    let glyphs = context.glyphs;
    let buffer = frame.buffer_mut();
    for band in 0..layout.bands.min(right.levels.len()) {
        let x = layout.x(band);
        for row in 0..rows {
            let style = zone_style(row, rows);
            let offset = saturating_u16(row) + 1;
            if let Some(symbol) = fill_glyph(left.levels[band], rows, row, glyphs.mirror_up) {
                buffer.set_string(x, axis_y - offset, layout.fill(symbol), style);
            }
            if let Some(symbol) = fill_glyph(right.levels[band], rows, row, glyphs.mirror_down) {
                buffer.set_string(x, axis_y + offset, layout.fill(symbol), style);
            }
        }
    }
    let axis = Rect::new(bars.x, axis_y, bars.width, AXIS_ROWS);
    let rule = glyphs.border.horizontal_top.repeat(usize::from(bars.width));
    buffer.set_string(axis.x, axis.y, rule, theme::MUTED);
    if context.is_compact() {
        return;
    }
    for (x, label) in layout.axis_labels(axis, &AXIS_FREQUENCIES, LABEL_PAD) {
        buffer.set_string(x, axis.y, label, theme::MUTED);
    }
    buffer.set_string(area.x, area.y + half / 2, LEFT_LABEL, theme::BOLD);
    buffer.set_string(area.x, axis_y + 1 + half / 2, RIGHT_LABEL, theme::BOLD);
}
