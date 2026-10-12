//! Encabezado: el marco con el nombre y el estado de la conexión, las pestañas y el
//! resumen de la cola.

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use super::chrome::{Context, block_title, end_title, titled_block};
use super::format::{countdown, format_duration, songs_count};
use super::gauges::{MeterCell, meter, meter_line};
use super::theme;
use super::widgets::{millis, spread};
use crate::tui::action::{key_label, view_action};
use crate::tui::state::{AppState, ConnectionState, ViewKind};

const APPLICATION_NAME: &str = "mpctui";
/// Esquinas del marco, a cada lado del borde superior.
const CORNERS: usize = 2;
/// Separación mínima entre el título de la izquierda y el de la derecha.
const MIN_TITLE_GAP: usize = 1;
const RETRY_METER_CELLS: usize = 8;
/// `(intento N)` se muestra desde el segundo intento.
const FIRST_SHOWN_ATTEMPT: u32 = 2;

/// Marco de toda la pantalla: `┌─ mpctui ──── ● localhost:6600 ─┐`.
pub fn frame_block(state: &AppState, context: &Context<'_>) -> Block<'static> {
    let glyphs = context.glyphs;
    let name = name_line(state, context);
    let used = CORNERS
        + block_title(name.clone(), glyphs).width()
        + MIN_TITLE_GAP
        + end_title(Line::default(), glyphs).width();
    let available = usize::from(context.width).saturating_sub(used);
    let connection = end_title(connection_line(state, context, available), glyphs);
    titled_block(name, glyphs).title_top(connection.right_aligned())
}

/// `mpctui`; con una actualización de la base en curso,
/// `mpctui ─ ◐ actualizando biblioteca 0:42` (compacto: `mpctui ◐`).
fn name_line(state: &AppState, context: &Context<'_>) -> Line<'static> {
    let glyphs = context.glyphs;
    let mut spans = vec![Span::styled(APPLICATION_NAME, theme::ACCENT)];
    let Some(started) = state.database_update else {
        return Line::from(spans);
    };
    let spinner = glyphs.spinner_frame(state.ticks);
    if context.is_compact() {
        spans.push(Span::styled(format!(" {spinner}"), theme::INFO));
    } else {
        let elapsed = format_duration(context.now.saturating_duration_since(started));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(glyphs.border.horizontal_top, theme::BORDER));
        spans.push(Span::styled(
            format!(" {spinner} actualizando biblioteca {elapsed}"),
            theme::INFO,
        ));
    }
    Line::from(spans)
}

/// Cuánto de la desconexión se muestra. Si no entra, se quita primero el intento,
/// después el medidor y por último `sin conexión, `.
#[derive(Debug, Clone, Copy)]
struct Detail {
    status_word: bool,
    meter: bool,
    attempt: bool,
}

const DETAIL_LEVELS: [Detail; 4] = [
    Detail {
        status_word: true,
        meter: true,
        attempt: true,
    },
    Detail {
        status_word: true,
        meter: true,
        attempt: false,
    },
    Detail {
        status_word: true,
        meter: false,
        attempt: false,
    },
    Detail {
        status_word: false,
        meter: false,
        attempt: false,
    },
];

/// `● localhost:6600`, o la desconexión con el detalle que entre en `available`.
/// En modo compacto, siempre la forma mínima.
fn connection_line(state: &AppState, context: &Context<'_>, available: usize) -> Line<'static> {
    if state.connection == ConnectionState::Connected {
        let text = format!("{} {}", context.glyphs.connected, state.server);
        return Line::styled(text, theme::INFO);
    }
    let levels = if context.is_compact() {
        &DETAIL_LEVELS[DETAIL_LEVELS.len() - 1..]
    } else {
        &DETAIL_LEVELS[..]
    };
    let mut lines: Vec<Line<'static>> = levels
        .iter()
        .map(|detail| disconnected_line(state, context, *detail))
        .collect();
    let shortest = lines.pop().unwrap_or_default();
    lines
        .into_iter()
        .find(|line| line.width() <= available)
        .unwrap_or(shortest)
}

/// `✖ sin conexión, reintento en 5 s ■■■□□□□□ (intento 4)` o
/// `✖ sin conexión, conectando ◐ (intento 5)`.
fn disconnected_line(state: &AppState, context: &Context<'_>, detail: Detail) -> Line<'static> {
    let glyphs = context.glyphs;
    let mut text = format!("{} ", glyphs.disconnected);
    if detail.status_word {
        text.push_str("sin conexión, ");
    }
    let mut extra = Vec::new();
    let attempt = match state.connection {
        ConnectionState::Connected => return Line::default(),
        ConnectionState::Waiting {
            attempt,
            retry_at,
            delay,
        } => {
            let remaining = retry_at.saturating_duration_since(context.now);
            text.push_str(&format!("reintento en {}", countdown(remaining)));
            if detail.meter {
                extra.push(Span::raw(" "));
                extra.extend(meter_line(&waited_meter(delay, remaining), glyphs).spans);
            }
            attempt
        }
        ConnectionState::Connecting { attempt } => {
            text.push_str(&format!("conectando {}", glyphs.spinner_frame(state.ticks)));
            attempt
        }
    };
    let mut spans = vec![Span::styled(text, theme::ERROR)];
    spans.extend(extra);
    if detail.attempt && attempt >= FIRST_SHOWN_ATTEMPT {
        spans.push(Span::styled(format!(" (intento {attempt})"), theme::ERROR));
    }
    Line::from(spans)
}

/// La parte ya esperada de la espera actual: se llena hasta que se reintenta.
fn waited_meter(delay: Duration, remaining: Duration) -> Vec<MeterCell> {
    let waited = delay.saturating_sub(remaining);
    meter(millis(waited), millis(delay), RETRY_METER_CELLS)
}

/// Pestañas (la activa entre corchetes) y, si entra, `cola: 1229 temas  65:04:13`.
pub fn render_tabs(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let mut tabs = vec![Span::raw(" ")];
    for (index, view) in state.views().into_iter().enumerate() {
        if index > 0 {
            tabs.push(Span::raw(" "));
        }
        let key = key_label(view_action(view), context.glyphs).unwrap_or_default();
        let label = format!("{key} {}", view_title(view, context.is_compact()));
        tabs.push(if view == state.view {
            Span::styled(format!("[{label}]"), theme::FOCUSED)
        } else {
            Span::styled(format!(" {label} "), theme::MUTED)
        });
    }
    let tabs = Line::from(tabs);
    let summary = queue_summary(state);
    let line = if tabs.width() + summary.width() < usize::from(area.width) {
        spread(tabs, summary, area.width, context.glyphs)
    } else {
        tabs
    };
    frame
        .buffer_mut()
        .set_line(area.x, area.y, &line, area.width);
}

/// Sin duración si falta la de algún tema.
fn queue_summary(state: &AppState) -> Line<'static> {
    let mut text = format!("cola: {}", songs_count(state.queue.items().len()));
    if let Some(total) = state.queue.timeline().total {
        text.push_str(&format!("  {}", format_duration(total)));
    }
    text.push(' ');
    Line::styled(text, theme::MUTED)
}

/// En modo compacto, abreviado.
pub fn view_title(view: ViewKind, abbreviated: bool) -> &'static str {
    match (view, abbreviated) {
        (ViewKind::Queue, _) => "Cola",
        (ViewKind::Library, false) => "Biblioteca",
        (ViewKind::Library, true) => "Bibl.",
        (ViewKind::Browse, false) => "Directorios",
        (ViewKind::Browse, true) => "Dir.",
        (ViewKind::Search, false) => "Búsqueda",
        (ViewKind::Search, true) => "Búsq.",
        (ViewKind::Playlists, false) => "Playlists",
        (ViewKind::Playlists, true) => "Playl.",
        (ViewKind::Visualizer, _) => "Visual",
        (ViewKind::Lyrics, _) => "Letra",
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::tui::appearance::ProgressBars;
    use crate::tui::glyphs::{ASCII, UNICODE};
    use crate::tui::views::layout::LayoutMode;

    fn context(mode: LayoutMode, now: Instant) -> Context<'static> {
        Context {
            glyphs: &ASCII,
            mode,
            width: 100,
            now,
            progress_bars: ProgressBars::default(),
        }
    }

    fn waiting(now: Instant, attempt: u32) -> AppState {
        let mut state = AppState::new("localhost:6600".into());
        state.connection = ConnectionState::Waiting {
            attempt,
            retry_at: now + Duration::from_secs(5),
            delay: Duration::from_secs(8),
        };
        state
    }

    #[test]
    fn retry_meter_fills_while_waiting() {
        let delay = Duration::from_secs(8);
        let filled = |remaining| {
            waited_meter(delay, remaining)
                .iter()
                .filter(|cell| **cell == MeterCell::Filled)
                .count()
        };
        assert_eq!(filled(delay), 0);
        assert_eq!(filled(delay / 2), RETRY_METER_CELLS / 2);
        assert_eq!(filled(Duration::ZERO), RETRY_METER_CELLS);
    }

    #[test]
    fn disconnection_detail_shrinks_in_order() {
        let now = Instant::now();
        let state = waiting(now, 4);
        let full = context(LayoutMode::Full, now);
        let text = |available| connection_line(&state, &full, available).to_string();
        let complete = "(x) sin conexión, reintento en 5 s [###-----] (intento 4)";
        assert_eq!(text(80), complete);
        assert_eq!(text(57), complete);
        assert_eq!(text(56), "(x) sin conexión, reintento en 5 s [###-----]");
        assert_eq!(text(40), "(x) sin conexión, reintento en 5 s");
        assert_eq!(text(20), "(x) reintento en 5 s");
        assert_eq!(text(5), "(x) reintento en 5 s");
        let compact = context(LayoutMode::Compact, now);
        let text = connection_line(&state, &compact, 80).to_string();
        assert_eq!(text, "(x) reintento en 5 s");
    }

    #[test]
    fn first_attempt_and_connecting() {
        let now = Instant::now();
        let full = context(LayoutMode::Full, now);
        let state = waiting(now, 1);
        let text = connection_line(&state, &full, 80).to_string();
        assert!(!text.contains("(intento"), "{text}");
        let mut state = waiting(now, 5);
        state.connection = ConnectionState::Connecting { attempt: 5 };
        let text = connection_line(&state, &full, 80).to_string();
        assert_eq!(text, "(x) sin conexión, conectando | (intento 5)");
    }

    #[test]
    fn database_update_shows_spinner_and_elapsed_time() {
        let now = Instant::now();
        let mut state = AppState::new("localhost:6600".into());
        state.database_update = Some(now - Duration::from_secs(42));
        let full = context(LayoutMode::Full, now);
        let text = name_line(&state, &full).to_string();
        assert_eq!(text, "mpctui - | actualizando biblioteca 0:42");
        let compact = Context {
            glyphs: &UNICODE,
            ..context(LayoutMode::Compact, now)
        };
        assert_eq!(name_line(&state, &compact).to_string(), "mpctui ◐");
    }
}
