//! Medidores: una barra de RMS por canal con el pico retenido, la escala en dBFS, la
//! correlación y una leyenda. En modo compacto, solo barras de una fila.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;

use super::correlation::correlation_line;
use crate::domain::{
    ChannelLevel, METER_FLOOR_DB, METER_PEAK_HOLD, RMS_WINDOW, VisualRequest, VisualizerStyle,
};
use crate::i18n::{Catalog, Message, Phrase};
use crate::tui::glyphs::Glyphs;
use crate::tui::views::chrome::Context;
use crate::tui::views::format::{MISSING, saturating_u16};
use crate::tui::views::theme;

/// Letra del canal y comienzo de la barra.
const LABEL_INDENT: u16 = 1;
const BAR_OFFSET: u16 = 4;
/// Columnas a la derecha para los números (`  -9.4 dB  pico   -3.1`).
const NUMBERS_WIDTH: u16 = 24;
const FULL_BAR_ROWS: u16 = 2;
const COMPACT_BAR_ROWS: u16 = 1;
/// Filas vacías entre bloques.
const GAP_ROWS: u16 = 1;
const STEREO_LABELS: [&str; 2] = ["L", "R"];
const MONO_LABEL: &str = "M";
/// Marcas de la escala, en dBFS.
const SCALE_MARKS: [f32; 8] = [-60.0, -40.0, -30.0, -20.0, -12.0, -6.0, -3.0, 0.0];
const SCALE_UNIT: &str = "dBFS";
/// Hasta acá verde; hasta `YELLOW_UNTIL_DB`, amarillo; más arriba, rojo.
const GREEN_UNTIL_DB: f32 = -12.0;
const YELLOW_UNTIL_DB: f32 = -3.0;
/// Separación mínima entre etiquetas de la escala.
const SCALE_LABEL_GAP: u16 = 1;

/// Los Medidores no dependen del tamaño.
pub fn request() -> VisualRequest {
    VisualRequest {
        style: VisualizerStyle::Meters,
        columns: 0,
        rows: 0,
    }
}

pub fn details(texts: &dyn Catalog) -> String {
    texts.phrase(Phrase::MetersDetail).to_owned()
}

/// Dónde va cada cosa a lo ancho.
#[derive(Debug, Clone, Copy)]
struct MeterLayout {
    bar_x: u16,
    /// Celdas de la escala de `METER_FLOOR_DB` a 0.
    scale_width: u16,
}

impl MeterLayout {
    fn new(area: Rect) -> Self {
        Self {
            bar_x: area.x + BAR_OFFSET.min(area.width),
            scale_width: area.width.saturating_sub(BAR_OFFSET + NUMBERS_WIDTH),
        }
    }

    /// Celdas (con fracción) desde el comienzo de la escala hasta `decibels`.
    fn position(self, decibels: f32) -> f32 {
        let fraction = (decibels - METER_FLOOR_DB) / -METER_FLOOR_DB;
        fraction.clamp(0.0, 1.0) * f32::from(self.scale_width)
    }

    /// dBFS en el centro de la celda `cell`.
    fn decibels_at(self, cell: u16) -> f32 {
        METER_FLOOR_DB + (f32::from(cell) + 0.5) / f32::from(self.scale_width) * -METER_FLOOR_DB
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    channels: &[ChannelLevel],
    correlation: Option<f32>,
    context: &Context<'_>,
) {
    let compact = context.is_compact();
    let layout = MeterLayout::new(area);
    let bar_rows = if compact {
        COMPACT_BAR_ROWS
    } else {
        FULL_BAR_ROWS
    };
    let labels: &[&str] = if channels.len() == 1 {
        &[MONO_LABEL]
    } else {
        &STEREO_LABELS
    };
    let glyphs = context.glyphs;
    let buffer = frame.buffer_mut();
    let mut y = area.y + if compact { 0 } else { GAP_ROWS };
    for (channel, label) in channels.iter().zip(labels) {
        for row in 0..bar_rows {
            if y + row >= area.bottom() {
                return;
            }
            draw_bar(buffer, layout, y + row, *channel, glyphs);
        }
        buffer.set_string(area.x + LABEL_INDENT, y, *label, theme::BOLD);
        let numbers_x = layout.bar_x + layout.scale_width + 2;
        let numbers = numbers(*channel, context.texts);
        buffer.set_string(numbers_x, y, numbers, theme::PLAIN);
        y += bar_rows + GAP_ROWS;
    }
    if compact {
        return;
    }
    let mut rows = vec![Row::Scale, Row::ScaleLabels, Row::Gap];
    if let Some(value) = correlation {
        rows.extend([Row::Correlation(value), Row::Gap]);
    }
    rows.push(Row::Legend);
    for row in rows {
        if y >= area.bottom() {
            return;
        }
        match row {
            Row::Scale => draw_scale(buffer, layout, y, glyphs),
            Row::ScaleLabels => draw_scale_labels(buffer, layout, y),
            Row::Correlation(value) => {
                let available = area.right().saturating_sub(layout.bar_x);
                let line = correlation_line(value, available, glyphs, context.texts);
                buffer.set_line(layout.bar_x, y, &line, available);
            }
            Row::Legend => {
                let legend = legend(glyphs, context.texts);
                buffer.set_string(layout.bar_x, y, legend, theme::MUTED);
            }
            Row::Gap => {}
        }
        y += 1;
    }
}

/// Filas debajo de las barras, en orden.
enum Row {
    Scale,
    ScaleLabels,
    Correlation(f32),
    Legend,
    Gap,
}

/// La barra del RMS, en octavos de celda si el juego los tiene, y el pico retenido.
fn draw_bar(
    buffer: &mut Buffer,
    layout: MeterLayout,
    y: u16,
    level: ChannelLevel,
    glyphs: &Glyphs,
) {
    let steps = glyphs.meter_eighths.len();
    let filled = (layout.position(level.rms) * steps as f32).round() as usize;
    let (full, partial) = (filled / steps, filled % steps);
    for cell in 0..full {
        let x = layout.bar_x + saturating_u16(cell);
        let style = zone_style(layout.decibels_at(saturating_u16(cell)));
        buffer.set_string(x, y, glyphs.meter_eighths[steps - 1], style);
    }
    if partial > 0 {
        let x = layout.bar_x + saturating_u16(full);
        let style = zone_style(layout.decibels_at(saturating_u16(full)));
        buffer.set_string(x, y, glyphs.meter_eighths[partial - 1], style);
    }
    if level.peak > METER_FLOOR_DB {
        let cell = (layout.position(level.peak) as u16).min(layout.scale_width);
        let style = zone_style(level.peak);
        buffer.set_string(layout.bar_x + cell, y, glyphs.meter_peak, style);
    }
}

/// Verde hasta -12, amarillo hasta -3 y rojo arriba.
fn zone_style(decibels: f32) -> Style {
    if decibels <= GREEN_UNTIL_DB {
        theme::SPECTRUM_LOW
    } else if decibels <= YELLOW_UNTIL_DB {
        theme::SPECTRUM_MIDDLE
    } else {
        theme::SPECTRUM_HIGH
    }
}

/// `  -9.4 dB  pico   -3.1`; en el piso, `—`.
fn numbers(level: ChannelLevel, texts: &dyn Catalog) -> String {
    format!(
        "{:>6} dB  {} {:>6}",
        decibels_text(level.rms),
        texts.phrase(Phrase::Peak),
        decibels_text(level.peak)
    )
}

fn decibels_text(decibels: f32) -> String {
    if decibels <= METER_FLOOR_DB {
        MISSING.to_owned()
    } else {
        format!("{decibels:.1}")
    }
}

fn mark_cell(layout: MeterLayout, mark: f32) -> u16 {
    layout.position(mark).round() as u16
}

/// `┴──────┴───┴ dBFS`.
fn draw_scale(buffer: &mut Buffer, layout: MeterLayout, y: u16, glyphs: &Glyphs) {
    let cells = usize::from(layout.scale_width) + 1;
    let rule = glyphs.border.horizontal_top.repeat(cells);
    buffer.set_string(layout.bar_x, y, rule, theme::MUTED);
    for mark in SCALE_MARKS {
        let x = layout.bar_x + mark_cell(layout, mark);
        buffer.set_string(x, y, glyphs.scale_tick, theme::MUTED);
    }
    let unit_x = layout.bar_x + saturating_u16(cells) + 1;
    buffer.set_string(unit_x, y, SCALE_UNIT, theme::MUTED);
}

/// Los valores de las marcas, cada uno desde su marca; el que se pisa se omite.
fn draw_scale_labels(buffer: &mut Buffer, layout: MeterLayout, y: u16) {
    let mut free_from = layout.bar_x;
    for mark in SCALE_MARKS {
        let label = format!("{mark:.0}");
        let x = layout.bar_x + mark_cell(layout, mark);
        if x < free_from {
            continue;
        }
        free_from = x + saturating_u16(Span::raw(&label).width()) + SCALE_LABEL_GAP;
        buffer.set_string(x, y, label, theme::MUTED);
    }
}

/// `barra = RMS (300 ms)   │ = pico retenido 1,5 s`.
fn legend(glyphs: &Glyphs, texts: &dyn Catalog) -> String {
    texts.message(&Message::MetersLegend {
        window: RMS_WINDOW,
        peak_glyph: glyphs.meter_peak,
        hold: METER_PEAK_HOLD,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{English, Spanish};
    use crate::tui::glyphs::UNICODE;

    #[test]
    fn the_scale_is_linear_in_decibels() {
        let layout = MeterLayout::new(Rect::new(0, 0, BAR_OFFSET + NUMBERS_WIDTH + 60, 1));
        assert_eq!(layout.position(METER_FLOOR_DB), 0.0);
        assert_eq!(layout.position(-30.0), 30.0);
        assert_eq!(layout.position(6.0), 60.0);
        assert!((layout.decibels_at(0) + 59.5).abs() < 1e-4);
    }

    #[test]
    fn numbers_show_a_dash_at_the_floor() {
        let level = ChannelLevel {
            rms: -9.43,
            peak: -3.1,
        };
        assert_eq!(numbers(level, &Spanish), "  -9.4 dB  pico   -3.1");
        assert_eq!(
            numbers(ChannelLevel::SILENT, &Spanish),
            "     — dB  pico      —"
        );
        assert_eq!(numbers(level, &English), "  -9.4 dB  peak   -3.1");
    }

    #[test]
    fn colors_change_at_minus_twelve_and_minus_three() {
        assert_eq!(zone_style(-12.0), theme::SPECTRUM_LOW);
        assert_eq!(zone_style(-6.0), theme::SPECTRUM_MIDDLE);
        assert_eq!(zone_style(-1.0), theme::SPECTRUM_HIGH);
    }

    #[test]
    fn the_legend_explains_the_bar_and_the_peak() {
        assert_eq!(
            legend(&UNICODE, &Spanish),
            "barra = RMS (300 ms)   │ = pico retenido 1,5 s"
        );
        assert_eq!(
            legend(&UNICODE, &English),
            "bar = RMS (300 ms)   │ = held peak 1.5 s"
        );
    }
}
