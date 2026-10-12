//! Línea inferior: el último aviso (durante `MESSAGE_DURATION`), las teclas de la
//! búsqueda mientras se escribe o los atajos más útiles de la vista activa.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::chrome::Context;
use super::theme;
use crate::tui::action::{Action, key_label};
use crate::tui::glyphs::Glyphs;
use crate::tui::prompt::PromptKind;
use crate::tui::state::{AppState, Notice, NoticeLevel, ViewKind};

/// Teclas de la línea de búsqueda (las maneja el prompt, no la tabla de atajos).
const SEARCH_KEYS: &str = "Enter busca   Tab cambia el campo   Esc cancela";
const HINT_GAP: &str = "   ";
/// Separación entre el borde de la pantalla y el texto.
const LEADING_SPACE: &str = " ";
/// Separación entre el prefijo de un aviso y su texto.
const PREFIX_GAP: &str = " ";

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let searching = state
        .prompt
        .as_ref()
        .is_some_and(|prompt| prompt.kind == PromptKind::Search);
    let notice = state
        .notice
        .as_ref()
        .filter(|notice| notice.is_visible_at(context.now));
    let line = match notice {
        Some(notice) => notice_line(notice, context.glyphs),
        None if searching => Line::styled(format!("{LEADING_SPACE}{SEARCH_KEYS}"), theme::MUTED),
        None => hints_line(state.view, context),
    };
    frame
        .buffer_mut()
        .set_line(area.x, area.y, &line, area.width);
}

/// Atajos que sugiere cada vista; la tecla sale de la tabla `BINDINGS`.
fn view_hints(view: ViewKind) -> &'static [(Action, &'static str)] {
    match view {
        ViewKind::Queue => &[
            (Action::Activate, "reproducir"),
            (Action::Delete, "borrar"),
            (Action::ShowPlaying, "ir al actual"),
            (Action::StartSearch, "buscar"),
            (Action::ToggleHelp, "ayuda"),
        ],
        ViewKind::Library => &[
            (Action::Forward, "entrar"),
            (Action::Back, "volver"),
            (Action::Add, "agregar"),
            (Action::AddAll, "agregar álbum"),
            (Action::ToggleHelp, "ayuda"),
        ],
        ViewKind::Browse => &[
            (Action::Activate, "entrar o agregar"),
            (Action::Back, "subir"),
            (Action::Add, "agregar"),
            (Action::ToggleHelp, "ayuda"),
        ],
        ViewKind::Search => &[
            (Action::StartSearch, "buscar"),
            (Action::Add, "agregar"),
            (Action::AddAll, "agregar todos"),
            (Action::ToggleHelp, "ayuda"),
        ],
        ViewKind::Playlists => &[
            (Action::Activate, "cargar"),
            (Action::Delete, "borrar"),
            (Action::SaveQueue, "guardar cola"),
            (Action::ToggleHelp, "ayuda"),
        ],
        ViewKind::Visualizer => &[
            (Action::ShowVisualizer, "siguiente forma"),
            (Action::TogglePause, "pausa"),
            (Action::ToggleHelp, "ayuda"),
        ],
    }
}

/// En modo compacto solo se sugiere la ayuda.
const COMPACT_HINTS: &[(Action, &str)] = &[(Action::ToggleHelp, "ayuda")];

fn hints_line(view: ViewKind, context: &Context<'_>) -> Line<'static> {
    let hints = if context.is_compact() {
        COMPACT_HINTS
    } else {
        view_hints(view)
    };
    let mut spans = vec![Span::raw(LEADING_SPACE)];
    for (index, (action, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(HINT_GAP));
        }
        let key = key_label(*action, context.glyphs).unwrap_or_default();
        spans.push(Span::styled(key, theme::ACCENT));
        spans.push(Span::styled(format!(" {label}"), theme::MUTED));
    }
    Line::from(spans)
}

/// El prefijo distingue el tipo de aviso sin depender del color.
fn notice_line<'a>(notice: &'a Notice, glyphs: &Glyphs) -> Line<'a> {
    let (prefix, style) = match notice.level {
        NoticeLevel::Info => (glyphs.message_info, theme::INFO),
        NoticeLevel::Error => (glyphs.message_error, theme::ERROR),
    };
    Line::from(vec![
        Span::raw(LEADING_SPACE),
        Span::styled(prefix, style),
        Span::raw(PREFIX_GAP),
        Span::styled(notice.text.as_str(), style),
    ])
}
