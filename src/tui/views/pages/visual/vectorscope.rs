//! Vectorscopio: los pares L/R en un área cuadrada con ejes M (vertical) y S
//! (horizontal), guías hacia L y R, y la correlación debajo.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::canvas::DotCanvas;
use super::correlation::correlation_line;
use crate::domain::{ScopePoint, VisualRequest, VisualizerStyle};
use crate::tui::glyphs::{CellPlot, Plot};
use crate::tui::views::chrome::Context;
use crate::tui::views::format::saturating_u16;
use crate::tui::views::theme;

const LABEL_ROWS: u16 = 1;
const CORRELATION_ROWS: u16 = 1;
/// Una celda mide 1 × 2: el cuadrado tiene el doble de columnas que de filas.
const COLUMNS_PER_ROW: u16 = 2;
/// Cada cuántos puntos se dibuja uno de las guías braille.
const GUIDE_DOT_SPACING: usize = 3;
/// Puntos por celda desde los que se pasa al nivel siguiente de densidad (ASCII).
const DENSITY_THRESHOLDS: [u32; 3] = [1, 3, 8];
const MID_LABEL: &str = "M";
const LEFT_LABEL: &str = "L";
const RIGHT_LABEL: &str = "R";
/// Separación entre las letras L/R y el cuadrado.
const LABEL_GAP: u16 = 1;

/// El Vectorscopio no depende del tamaño: los puntos ya vienen escalados.
pub fn request() -> VisualRequest {
    VisualRequest {
        style: VisualizerStyle::Vectorscope,
        columns: 0,
        rows: 0,
    }
}

/// `M/S  2048 muestras`.
pub fn details(points: usize) -> String {
    format!("{MID_LABEL}/S  {points} muestras")
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    points: &[ScopePoint],
    correlation: f32,
    context: &Context<'_>,
) {
    let compact = context.is_compact();
    let (labels, scope_rows, correlation_y) = if compact {
        (None, area.height, None)
    } else {
        let rows = area.height.saturating_sub(LABEL_ROWS + CORRELATION_ROWS);
        (Some(area.y), rows, Some(area.bottom().saturating_sub(1)))
    };
    let side = scope_rows.min(area.width / COLUMNS_PER_ROW);
    if side == 0 {
        return;
    }
    let width = side * COLUMNS_PER_ROW;
    let top = area.y + labels.map_or(0, |_| LABEL_ROWS);
    let square = Rect::new(area.x + (area.width - width) / 2, top, width, side);
    let buffer = frame.buffer_mut();
    match context.glyphs.plot {
        Plot::Braille => draw_braille(buffer, square, points),
        Plot::Cells(cells) => draw_cells(buffer, square, points, cells),
    }
    if let Some(y) = labels {
        draw_labels(buffer, area, square, y);
    }
    if let Some(y) = correlation_y {
        let line = correlation_line(correlation, area.width, context.glyphs);
        let x = area.x + area.width.saturating_sub(saturating_u16(line.width())) / 2;
        buffer.set_line(x, y, &line, area.width);
    }
}

/// Columna y fila de un punto en una grilla de `columns × rows`.
fn grid_position(point: ScopePoint, columns: usize, rows: usize) -> (usize, usize) {
    let scale = |value: f32, cells: usize| {
        ((value.clamp(-1.0, 1.0) + 1.0) / 2.0 * cells.saturating_sub(1) as f32).round() as usize
    };
    (scale(point.side, columns), scale(-point.mid, rows))
}

/// Guías punteadas en las diagonales y los puntos encima.
fn draw_braille(buffer: &mut Buffer, square: Rect, points: &[ScopePoint]) {
    let mut guides = DotCanvas::new(square, Plot::Braille);
    let (columns, rows) = (guides.columns(), guides.rows());
    for dot in (0..rows.min(columns)).step_by(GUIDE_DOT_SPACING) {
        guides.set(dot, dot);
        guides.set(columns - 1 - dot, dot);
    }
    guides.draw(buffer, theme::SCOPE_GUIDE);
    let mut signal = DotCanvas::new(square, Plot::Braille);
    for point in points {
        let (column, row) = grid_position(*point, columns, rows);
        signal.set(column, row);
    }
    signal.draw(buffer, theme::SCOPE);
}

/// Guías `\` y `/` y, encima, la densidad de puntos de cada celda.
fn draw_cells(buffer: &mut Buffer, square: Rect, points: &[ScopePoint], cells: CellPlot) {
    for row in 0..square.height {
        let offset = row * COLUMNS_PER_ROW;
        let y = square.y + row;
        buffer.set_string(square.x + offset, y, cells.guide_left, theme::SCOPE_GUIDE);
        let right = square.right() - 1 - offset;
        buffer.set_string(right, y, cells.guide_right, theme::SCOPE_GUIDE);
    }
    let (columns, rows) = (usize::from(square.width), usize::from(square.height));
    let mut counts = vec![0_u32; columns * rows];
    for point in points {
        let (column, row) = grid_position(*point, columns, rows);
        counts[row * columns + column] += 1;
    }
    for (index, count) in counts.into_iter().enumerate() {
        let level = DENSITY_THRESHOLDS
            .iter()
            .filter(|threshold| count >= **threshold)
            .count();
        if level == 0 {
            continue;
        }
        let x = square.x + saturating_u16(index % columns);
        let y = square.y + saturating_u16(index / columns);
        buffer.set_string(x, y, cells.density[level], theme::SCOPE);
    }
}

/// `M` arriba al centro; `L` y `R` en las puntas de las guías.
fn draw_labels(buffer: &mut Buffer, area: Rect, square: Rect, y: u16) {
    let middle = square.x + square.width / 2;
    buffer.set_string(middle, y, MID_LABEL, theme::BOLD);
    if let Some(x) = square.x.checked_sub(LABEL_GAP + 1).filter(|x| *x >= area.x) {
        buffer.set_string(x, y, LEFT_LABEL, theme::BOLD);
    }
    let right = square.right() + LABEL_GAP;
    if right < area.right() {
        buffer.set_string(right, y, RIGHT_LABEL, theme::BOLD);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_goes_up_and_side_goes_right() {
        let top = ScopePoint {
            side: 0.0,
            mid: 1.0,
        };
        assert_eq!(grid_position(top, 9, 9), (4, 0));
        let right = ScopePoint {
            side: 1.0,
            mid: -1.0,
        };
        assert_eq!(grid_position(right, 9, 9), (8, 8));
    }
}
