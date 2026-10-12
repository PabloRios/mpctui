//! Vista Visual (`6`): fila de título con la forma activa y el indicador de páginas, y
//! el dibujo de la forma. Cada forma dice qué resolución necesita según su área; la
//! vista lo deja en el estado para que la app se lo pida al visualizador. Si no se
//! puede leer el FIFO, explica cómo configurarlo.

mod bars;
mod canvas;
mod columns;
mod correlation;
mod meters;
mod mirror;
mod vectorscope;
mod waterfall;
mod wave;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use super::{draw_line, text_row};
use crate::domain::{VisualContent, VisualFrame, VisualRequest, VisualizerStyle};
use crate::i18n::{Catalog, Message, Phrase};
use crate::repository::Error;
use crate::service::VisualState;
use crate::tui::glyphs::Glyphs;
use crate::tui::state::{AppState, VisualView};
use crate::tui::views::chrome::Context;
use crate::tui::views::format::visualizer_style_name;
use crate::tui::views::theme;
use crate::tui::views::widgets::{centered_message, spread};

const TITLE_ROWS: u16 = 1;
const PART_GAP: &str = "  ";
/// Entre los puntos del indicador de páginas.
const PAGE_GAP: &str = " ";

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let title_rows = if context.is_compact() { 0 } else { TITLE_ROWS };
    let [title, body] =
        Layout::vertical([Constraint::Length(title_rows), Constraint::Fill(1)]).areas(area);
    let view = &mut state.visual;
    let content_area = text_row(body);
    view.request = Some(request(view.style, content_area, context));
    if !title.is_empty() {
        draw_title(frame, text_row(title), view, context);
    }
    match &view.state {
        VisualState::Frame(visual) => draw_frame(frame, content_area, visual, view.style, context),
        VisualState::Unavailable(error) => draw_help(frame, body, view, error, context.texts),
    }
}

/// Qué tiene que calcular el visualizador para dibujar `style` en `area`.
fn request(style: VisualizerStyle, area: Rect, context: &Context<'_>) -> VisualRequest {
    match style {
        VisualizerStyle::Bars => bars::request(area, context),
        VisualizerStyle::Mirror => mirror::request(area, context),
        VisualizerStyle::Wave => wave::request(area, context),
        VisualizerStyle::Waterfall => waterfall::request(area, context),
        VisualizerStyle::Vectorscope => vectorscope::request(),
        VisualizerStyle::Meters => meters::request(),
    }
}

/// Un cuadro de otra forma (recién cambiada) no se dibuja: el próximo ya es el bueno.
fn draw_frame(
    frame: &mut Frame,
    area: Rect,
    visual: &VisualFrame,
    style: VisualizerStyle,
    context: &Context<'_>,
) {
    if area.is_empty() || visual.content.style() != style {
        return;
    }
    if !visual.signal && visual.content.is_flat() {
        let no_signal = context.phrase(Phrase::NoSignal);
        return centered_message(frame, area, Line::styled(no_signal, theme::MUTED));
    }
    match &visual.content {
        VisualContent::Bars(bands) => bars::render(frame, area, bands, context),
        VisualContent::Mirror { left, right } => mirror::render(frame, area, left, right, context),
        VisualContent::Wave { columns, .. } => wave::render(frame, area, columns, context),
        VisualContent::Waterfall { columns } => waterfall::render(frame, area, columns, context),
        VisualContent::Vectorscope {
            points,
            correlation,
        } => vectorscope::render(frame, area, points, *correlation, context),
        VisualContent::Meters {
            channels,
            correlation,
        } => meters::render(frame, area, channels, *correlation, context),
    }
}

/// `Visualizador: Barras  ● ○ ○ ○ ○ ○  1/6` y, a la derecha, los datos de la forma.
/// Sin FIFO, solo `Visualizador` y `sin señal`.
fn draw_title(frame: &mut Frame, area: Rect, view: &VisualView, context: &Context<'_>) {
    let glyphs = context.glyphs;
    let title = context.phrase(Phrase::VisualizerTitle);
    let (left, details) = match &view.state {
        VisualState::Unavailable(_) => (
            Line::styled(title, theme::BOLD),
            context.phrase(Phrase::NoSignal).to_owned(),
        ),
        VisualState::Frame(visual) => {
            let style = context.phrase(visualizer_style_name(view.style));
            let name = format!("{title}: {style}");
            let left = Line::from(vec![
                Span::styled(name, theme::BOLD),
                Span::raw(PART_GAP),
                Span::styled(page_indicator(view, glyphs), theme::MUTED),
            ]);
            let details = if visual.content.style() == view.style {
                details(&visual.content, view.request, context.texts)
            } else {
                String::new()
            };
            (left, details)
        }
    };
    let line = spread(
        left,
        Line::styled(details, theme::MUTED),
        area.width,
        glyphs,
    );
    draw_line(frame, area, &line);
}

/// `● ○ ○ ○ ○ ○  1/6` entre las formas disponibles con los canales del FIFO.
fn page_indicator(view: &VisualView, glyphs: &Glyphs) -> String {
    let available = VisualizerStyle::available(view.channels);
    let position = available
        .iter()
        .position(|style| *style == view.style)
        .unwrap_or_default();
    let pages: Vec<&str> = (0..available.len())
        .map(|page| {
            if page == position {
                glyphs.page_active
            } else {
                glyphs.page_inactive
            }
        })
        .collect();
    format!(
        "{}{PART_GAP}{}/{}",
        pages.join(PAGE_GAP),
        position + 1,
        available.len()
    )
}

fn details(content: &VisualContent, request: Option<VisualRequest>, texts: &dyn Catalog) -> String {
    match content {
        VisualContent::Bars(bands) => bars::details(bands, texts),
        VisualContent::Mirror { left, .. } => mirror::details(left, texts),
        VisualContent::Wave { synced, span, .. } => wave::details(*synced, *span, texts),
        VisualContent::Waterfall { .. } => request
            .map(|request| waterfall::details(request, texts))
            .unwrap_or_default(),
        VisualContent::Vectorscope { points, .. } => vectorscope::details(points.len(), texts),
        VisualContent::Meters { .. } => meters::details(texts),
    }
}

/// Qué falta y cómo configurarlo.
fn draw_help(frame: &mut Frame, area: Rect, view: &VisualView, error: &Error, texts: &dyn Catalog) {
    let source = &view.source;
    let cannot_read = texts.message(&Message::CannotRead { source, error });
    let lines = [
        String::new(),
        format!("  {cannot_read}"),
        String::new(),
        format!("  {}", texts.phrase(Phrase::FifoAddOutput)),
        String::new(),
        "      audio_output {".to_owned(),
        "          type   \"fifo\"".to_owned(),
        format!(
            "          name   \"{}\"",
            texts.phrase(Phrase::FifoOutputName)
        ),
        format!("          path   \"{source}\""),
        format!("          format \"{}\"", view.format),
        "      }".to_owned(),
        String::new(),
        format!("  {}", texts.phrase(Phrase::FifoOtherPath)),
    ];
    let rows = Layout::vertical(vec![Constraint::Length(1); lines.len()]).split(text_row(area));
    for (text, row) in lines.into_iter().zip(rows.iter()) {
        draw_line(frame, *row, &Line::raw(text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::STEREO_CHANNELS;
    use crate::tui::glyphs::{ASCII, UNICODE};

    fn view(style: VisualizerStyle, channels: u8) -> VisualView {
        VisualView {
            style,
            channels,
            ..VisualView::default()
        }
    }

    #[test]
    fn the_indicator_marks_the_active_style() {
        let bars = view(VisualizerStyle::Bars, STEREO_CHANNELS);
        assert_eq!(page_indicator(&bars, &UNICODE), "● ○ ○ ○ ○ ○  1/6");
        let waterfall = view(VisualizerStyle::Waterfall, STEREO_CHANNELS);
        assert_eq!(page_indicator(&waterfall, &ASCII), ". . . * . .  4/6");
        let mono = view(VisualizerStyle::Meters, 1);
        assert_eq!(page_indicator(&mono, &ASCII), ". . . *  4/4");
    }
}
