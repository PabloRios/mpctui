//! Cascada (espectrograma): una banda por fila (agudos arriba), una columna por
//! instante (la más nueva a la derecha) y la intensidad como sombreado.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

use super::columns::nearest_band;
use crate::domain::{VisualRequest, VisualizerStyle, WATERFALL_COLUMNS_PER_SECOND, band_centers};
use crate::i18n::{Catalog, Message, Phrase};
use crate::tui::views::chrome::Context;
use crate::tui::views::format::{frequency_tick, saturating_u16};
use crate::tui::views::theme;

/// Etiquetas de frecuencia a la izquierda y su separación de los datos.
const LABEL_WIDTH: u16 = 4;
const LABEL_GAP: u16 = 1;
const AXIS_ROWS: u16 = 1;
/// Frecuencias del eje vertical, en Hz.
const AXIS_FREQUENCIES: [u32; 5] = [16_000, 4_000, 1_000, 250, 63];
/// Segundos del eje de tiempo: `-1 s`, `-3 s`, `-5 s`…
const FIRST_TIME_LABEL: u32 = 1;
const TIME_LABEL_STEP: u32 = 2;
/// Separación mínima entre etiquetas de tiempo.
const TIME_LABEL_GAP: u16 = 1;

/// Área de los datos: sin las etiquetas ni el eje de tiempo en modo completo.
fn data_area(area: Rect, context: &Context<'_>) -> Rect {
    if context.is_compact() {
        return area;
    }
    let margin = (LABEL_WIDTH + LABEL_GAP).min(area.width);
    Rect {
        x: area.x + margin,
        width: area.width - margin,
        height: area.height.saturating_sub(AXIS_ROWS),
        ..area
    }
}

/// Una columna por celda y una banda por fila.
pub fn request(area: Rect, context: &Context<'_>) -> VisualRequest {
    let data = data_area(area, context);
    VisualRequest {
        style: VisualizerStyle::Waterfall,
        columns: usize::from(data.width),
        rows: usize::from(data.height),
    }
}

/// `14 bandas  últimos 9 s`.
pub fn details(request: VisualRequest, texts: &dyn Catalog) -> String {
    let seconds = request.columns / WATERFALL_COLUMNS_PER_SECOND as usize;
    texts.message(&Message::WaterfallDetail {
        bands: request.rows,
        seconds,
    })
}

pub fn render(frame: &mut Frame, area: Rect, columns: &[Vec<f32>], context: &Context<'_>) {
    let data = data_area(area, context);
    let shades = context.glyphs.shades;
    let visible = columns.len().min(usize::from(data.width));
    let first_x = data.right() - saturating_u16(visible);
    let buffer = frame.buffer_mut();
    for (offset, levels) in columns[columns.len() - visible..].iter().enumerate() {
        let x = first_x + saturating_u16(offset);
        let bands = levels.len().min(usize::from(data.height));
        for (row, level) in levels.iter().rev().take(bands).enumerate() {
            let shade = shade_index(*level, shades.len());
            if shade > 0 {
                let y = data.y + saturating_u16(row);
                buffer.set_string(x, y, shades[shade], shade_style(*level));
            }
        }
    }
    if context.is_compact() {
        return;
    }
    let bands = columns.last().map_or(usize::from(data.height), Vec::len);
    draw_frequency_labels(frame, area, data, bands);
    draw_time_axis(frame, data, context.phrase(Phrase::Now));
}

/// Sombreado de un nivel: 0 es vacío y el último, el más intenso.
fn shade_index(level: f32, shades: usize) -> usize {
    (level.clamp(0.0, 1.0) * (shades - 1) as f32).round() as usize
}

/// De gris oscuro a cian, amarillo y rojo.
fn shade_style(level: f32) -> Style {
    let zones = theme::WATERFALL_SHADES.len();
    let zone = ((level.clamp(0.0, 1.0) * zones as f32) as usize).min(zones - 1);
    theme::WATERFALL_SHADES[zone]
}

/// `16k`, `4k`… alineadas a la derecha en la fila de la banda más cercana.
fn draw_frequency_labels(frame: &mut Frame, area: Rect, data: Rect, bands: usize) {
    let centers = band_centers(bands);
    let mut used_rows = Vec::new();
    for hertz in AXIS_FREQUENCIES {
        let band = nearest_band(&centers, hertz as f32);
        let row = bands.saturating_sub(1 + band);
        if row >= usize::from(data.height) || used_rows.contains(&row) {
            continue;
        }
        used_rows.push(row);
        let label = format!(
            "{:>width$}",
            frequency_tick(hertz),
            width = usize::from(LABEL_WIDTH)
        );
        let y = data.y + saturating_u16(row);
        frame
            .buffer_mut()
            .set_string(area.x, y, label, theme::MUTED);
    }
}

/// `-9 s … -1 s ahora`: cada etiqueta termina en la columna de ese instante.
fn draw_time_axis(frame: &mut Frame, data: Rect, now_label: &str) {
    let y = data.bottom();
    let now_width = saturating_u16(Span::raw(now_label).width());
    let Some(now_x) = data.right().checked_sub(now_width).filter(|x| *x >= data.x) else {
        return;
    };
    let buffer = frame.buffer_mut();
    buffer.set_string(now_x, y, now_label, theme::MUTED);
    let mut free_until = now_x.saturating_sub(TIME_LABEL_GAP);
    let column_width = WATERFALL_COLUMNS_PER_SECOND;
    for seconds in (FIRST_TIME_LABEL..).step_by(TIME_LABEL_STEP as usize) {
        let label = format!("-{seconds} s");
        let width = saturating_u16(Span::raw(&label).width());
        let Some(end) = data
            .right()
            .checked_sub(saturating_u16((seconds * column_width) as usize))
        else {
            break;
        };
        let Some(start) = (end + 1)
            .checked_sub(width)
            .filter(|start| *start >= data.x)
        else {
            break;
        };
        if end >= free_until {
            continue;
        }
        buffer.set_string(start, y, &label, theme::MUTED);
        free_until = start.saturating_sub(TIME_LABEL_GAP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{English, Spanish};
    use crate::tui::glyphs::{ASCII, UNICODE};

    #[test]
    fn shades_cover_every_level() {
        assert_eq!(shade_index(0.0, UNICODE.shades.len()), 0);
        assert_eq!(shade_index(1.0, UNICODE.shades.len()), 4);
        assert_eq!(shade_index(0.5, ASCII.shades.len()), 5);
        assert_eq!(shade_style(0.1), theme::WATERFALL_SHADES[0]);
        assert_eq!(shade_style(1.0), theme::WATERFALL_SHADES[3]);
    }

    #[test]
    fn details_count_bands_and_seconds() {
        let request = VisualRequest {
            style: VisualizerStyle::Waterfall,
            columns: 90,
            rows: 14,
        };
        assert_eq!(details(request, &Spanish), "14 bandas  últimos 9 s");
        assert_eq!(details(request, &English), "14 bands  last 9 s");
    }
}
