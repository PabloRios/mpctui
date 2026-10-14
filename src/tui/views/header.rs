//! Encabezado: el marco con el nombre y el estado de la conexión, las pestañas y el
//! resumen de la cola.

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use super::chrome::{Context, block_title, end_title, titled_block};
use super::format::{countdown, format_duration, numbers_stand_out};
use super::gauges::{MeterCell, MeterFill, meter, meter_line};
use super::widgets::{millis, spread};
use crate::i18n::{Message, Phrase};
use crate::tui::action::view_action;
use crate::tui::state::{AppState, ConnectionState, ViewKind};
use crate::tui::theme::Role;

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
    let name = name_line(state, context);
    let used = CORNERS
        + block_title(name.clone(), context).width()
        + MIN_TITLE_GAP
        + end_title(Line::default(), context).width();
    let available = usize::from(context.width).saturating_sub(used);
    let connection = end_title(connection_line(state, context, available), context);
    titled_block(name, Role::Subtle, context).title_top(connection.right_aligned())
}

/// `mpctui`; con una actualización de la base en curso,
/// `mpctui ─ ◐ actualizando biblioteca 0:42` (compacto: `mpctui ◐`).
fn name_line(state: &AppState, context: &Context<'_>) -> Line<'static> {
    let glyphs = context.glyphs;
    let mut spans = vec![Span::styled(APPLICATION_NAME, context.strong(Role::Accent))];
    let Some(started) = state.database_update else {
        return Line::from(spans);
    };
    let spinner = glyphs.spinner_frame(state.ticks);
    let updating_style = context.style(Role::Warning);
    if context.is_compact() {
        spans.push(Span::styled(format!(" {spinner}"), updating_style));
    } else {
        let elapsed = format_duration(context.now.saturating_duration_since(started));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            glyphs.border.horizontal_top,
            context.style(Role::Subtle),
        ));
        let updating = context.message(Message::UpdatingLibrary { elapsed: &elapsed });
        spans.push(Span::styled(
            format!(" {spinner} {updating}"),
            updating_style,
        ));
    }
    Line::from(spans)
}

/// Entre el servidor y la partición en uso.
const PARTITION_SEPARATOR: &str = " · ";

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

/// `● localhost:6600` (con `· cocina` si mpctui usa otra partición que `default`), o la
/// desconexión con el detalle que entre en `available`. En modo compacto, siempre la
/// forma mínima.
fn connection_line(state: &AppState, context: &Context<'_>, available: usize) -> Line<'static> {
    if state.connection == ConnectionState::Connected {
        return connected_line(state, context);
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

/// `●` en `success`, el servidor en `muted` y la partición en `accent_alt`.
fn connected_line(state: &AppState, context: &Context<'_>) -> Line<'static> {
    let mut spans = vec![
        Span::styled(context.glyphs.connected, context.style(Role::Success)),
        Span::styled(format!(" {}", state.server), context.style(Role::Muted)),
    ];
    if !state.partition.is_default() {
        spans.push(Span::styled(
            PARTITION_SEPARATOR,
            context.style(Role::Muted),
        ));
        spans.push(Span::styled(
            state.partition.to_string(),
            context.style(Role::AccentAlt),
        ));
    }
    Line::from(spans)
}

/// `✖ sin conexión, reintento en 5 s ■■■□□□□□ (intento 4)` o
/// `✖ sin conexión, conectando ◐ (intento 5)`.
fn disconnected_line(state: &AppState, context: &Context<'_>, detail: Detail) -> Line<'static> {
    let glyphs = context.glyphs;
    let mut text = format!("{} ", glyphs.disconnected);
    if detail.status_word {
        text.push_str(context.phrase(Phrase::Disconnected));
        text.push_str(", ");
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
            let countdown = countdown(remaining);
            text.push_str(&context.message(Message::RetryIn {
                countdown: &countdown,
            }));
            if detail.meter {
                extra.push(Span::raw(" "));
                let fill = MeterFill::Role(Role::Warning);
                let meter = meter_line(&waited_meter(delay, remaining), fill, context);
                extra.extend(meter.spans);
            }
            attempt
        }
        ConnectionState::Connecting { attempt } => {
            let connecting = context.phrase(Phrase::Connecting);
            let spinner = glyphs.spinner_frame(state.ticks);
            text.push_str(&format!("{connecting} {spinner}"));
            attempt
        }
    };
    let error = context.style(Role::Error);
    let mut spans = vec![Span::styled(text, error)];
    spans.extend(extra);
    if detail.attempt && attempt >= FIRST_SHOWN_ATTEMPT {
        let attempt = context.message(Message::Attempt(attempt));
        spans.push(Span::styled(format!(" {attempt}"), error));
    }
    Line::from(spans)
}

/// La parte ya esperada de la espera actual: se llena hasta que se reintenta.
fn waited_meter(delay: Duration, remaining: Duration) -> Vec<MeterCell> {
    let waited = delay.saturating_sub(remaining);
    meter(millis(waited), millis(delay), RETRY_METER_CELLS)
}

/// Pestañas (la activa entre corchetes) y, si entra, `cola: 1229 temas  65:04:13`.
/// Con los títulos completos que no entran (o en compacto), van los abreviados.
pub fn render_tabs(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let full = tabs_line(state, context, context.is_compact());
    let tabs = if full.width() > usize::from(area.width) {
        tabs_line(state, context, true)
    } else {
        full
    };
    let summary = queue_summary(state, context);
    let line = if tabs.width() + summary.width() < usize::from(area.width) {
        spread(tabs, summary, area.width, context.glyphs)
    } else {
        tabs
    };
    frame
        .buffer_mut()
        .set_line(area.x, area.y, &line, area.width);
}

fn tabs_line(state: &AppState, context: &Context<'_>, abbreviated: bool) -> Line<'static> {
    let mut tabs = vec![Span::raw(" ")];
    for (index, view) in state.views().into_iter().enumerate() {
        if index > 0 {
            tabs.push(Span::raw(" "));
        }
        let key = context.key(view_action(view));
        let title = context.phrase(view_title(view, abbreviated));
        let label = format!("{key} {title}");
        tabs.push(if view == state.view {
            Span::styled(format!("[{label}]"), context.style(Role::TabActive))
        } else {
            Span::styled(format!(" {label} "), context.style(Role::Muted))
        });
    }
    Line::from(tabs)
}

/// Sin duración si falta la de algún tema. Los números resaltan sobre el texto.
fn queue_summary(state: &AppState, context: &Context<'_>) -> Line<'static> {
    let mut text = context.message(Message::QueueSummary(state.queue.items().len()));
    if let Some(total) = state.queue.timeline().total {
        text.push_str(&format!("  {}", format_duration(total)));
    }
    text.push(' ');
    numbers_stand_out(&text, context.style(Role::Muted), context.style(Role::Text))
}

/// En modo compacto, abreviado.
pub fn view_title(view: ViewKind, abbreviated: bool) -> Phrase {
    match (view, abbreviated) {
        (ViewKind::Queue, _) => Phrase::ViewQueue,
        (ViewKind::Library, false) => Phrase::ViewLibrary,
        (ViewKind::Library, true) => Phrase::ViewLibraryShort,
        (ViewKind::Browse, false) => Phrase::ViewBrowse,
        (ViewKind::Browse, true) => Phrase::ViewBrowseShort,
        (ViewKind::Search, false) => Phrase::ViewSearch,
        (ViewKind::Search, true) => Phrase::ViewSearchShort,
        (ViewKind::Playlists, false) => Phrase::ViewPlaylists,
        (ViewKind::Playlists, true) => Phrase::ViewPlaylistsShort,
        (ViewKind::Visualizer, _) => Phrase::ViewVisualizer,
        (ViewKind::Lyrics, _) => Phrase::ViewLyrics,
        (ViewKind::Outputs, false) => Phrase::ViewOutputs,
        (ViewKind::Outputs, true) => Phrase::ViewOutputsShort,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::i18n::English;
    use crate::tui::glyphs::{ASCII, UNICODE};
    use crate::tui::views::chrome::testing;
    use crate::tui::views::layout::LayoutMode;

    fn context(mode: LayoutMode, now: Instant) -> Context<'static> {
        Context {
            mode,
            now,
            ..testing::context(&ASCII)
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
        let english = Context {
            texts: &English,
            ..full
        };
        let text = connection_line(&state, &english, 80).to_string();
        assert_eq!(text, "(x) disconnected, connecting | (attempt 5)");
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
