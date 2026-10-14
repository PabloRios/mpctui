//! Onda (osciloscopio): el segmento entre el mínimo y el máximo de cada columna de
//! puntos, con la escala `+1`, `0`, `-1` a la izquierda.

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::Rect;

use super::canvas::{DotCanvas, dots_per_cell};
use crate::domain::{SampleRange, VisualRequest, VisualizerStyle};
use crate::i18n::{Catalog, Message};
use crate::tui::theme::{GradientRole, Role};
use crate::tui::views::chrome::Context;

/// Columnas a la izquierda para la escala.
const LABEL_MARGIN: u16 = 3;
const TOP_LABEL: &str = "+1";
const MIDDLE_LABEL: &str = " 0";
const BOTTOM_LABEL: &str = "-1";
/// Desde esta amplitud una columna se marca como recorte.
const CLIP_THRESHOLD: f32 = 0.98;

/// Área de los puntos: sin el margen de la escala en modo completo.
fn plot_area(area: Rect, context: &Context<'_>) -> Rect {
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

/// Una columna de datos por columna de puntos.
pub fn request(area: Rect, context: &Context<'_>) -> VisualRequest {
    let (dot_columns, _) = dots_per_cell(context.glyphs.plot);
    VisualRequest {
        style: VisualizerStyle::Wave,
        columns: usize::from(plot_area(area, context).width) * dot_columns,
        rows: 0,
    }
}

/// `23 ms  sincronizado` o `23 ms  libre`.
pub fn details(synced: bool, span: Duration, texts: &dyn Catalog) -> String {
    texts.message(&Message::WaveDetail { span, synced })
}

pub fn render(frame: &mut Frame, area: Rect, columns: &[SampleRange], context: &Context<'_>) {
    let plot = plot_area(area, context);
    let mut canvas = DotCanvas::new(plot, context.glyphs.plot);
    let rows = canvas.rows();
    for (column, range) in columns.iter().take(canvas.columns()).enumerate() {
        for row in dot_row(range.high, rows)..=dot_row(range.low, rows) {
            canvas.set(column, row);
        }
    }
    let (dot_columns, _) = dots_per_cell(context.glyphs.plot);
    let amplitudes = cell_amplitudes(columns, dot_columns);
    let buffer = frame.buffer_mut();
    canvas.draw(buffer, |cell| {
        let amplitude = amplitudes.get(cell.column).copied().unwrap_or_default();
        if amplitude >= CLIP_THRESHOLD {
            context.style(Role::Error)
        } else {
            context.gradient(GradientRole::Wave, amplitude)
        }
    });
    if context.is_compact() || area.height == 0 {
        return;
    }
    let labels = [
        (area.y, TOP_LABEL),
        (area.y + area.height / 2, MIDDLE_LABEL),
        (area.bottom() - 1, BOTTOM_LABEL),
    ];
    for (y, label) in labels {
        buffer.set_string(area.x, y, label, context.style(Role::Muted));
    }
}

/// La amplitud máxima (`|x|`) de cada celda: elige su color y deja ver los recortes.
fn cell_amplitudes(columns: &[SampleRange], dot_columns: usize) -> Vec<f32> {
    columns
        .chunks(dot_columns.max(1))
        .map(|cell| {
            cell.iter()
                .map(|range| range.low.abs().max(range.high.abs()))
                .fold(0.0, f32::max)
        })
        .collect()
}

/// Fila de puntos de un valor entre -1 (abajo) y 1 (arriba).
fn dot_row(value: f32, rows: usize) -> usize {
    let span = rows.saturating_sub(1) as f32;
    ((1.0 - value.clamp(-1.0, 1.0)) / 2.0 * span).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{English, Spanish};

    #[test]
    fn values_map_from_top_to_bottom() {
        assert_eq!(dot_row(1.0, 9), 0);
        assert_eq!(dot_row(0.0, 9), 4);
        assert_eq!(dot_row(-1.0, 9), 8);
        assert_eq!(dot_row(-3.0, 9), 8);
    }

    #[test]
    fn each_cell_takes_the_loudest_of_its_columns() {
        let range = |low, high| SampleRange { low, high };
        let columns = [range(-0.2, 0.1), range(-0.5, 0.3), range(0.0, 0.99)];
        assert_eq!(cell_amplitudes(&columns, 2), [0.5, 0.99]);
        assert!(cell_amplitudes(&columns, 2)[1] >= CLIP_THRESHOLD);
    }

    #[test]
    fn details_say_whether_the_wave_is_synced() {
        let span = Duration::from_millis(23);
        assert_eq!(details(true, span, &Spanish), "23 ms  sincronizado");
        assert_eq!(details(false, span, &Spanish), "23 ms  libre");
        assert_eq!(details(true, span, &English), "23 ms  synced");
    }
}
