//! Línea inferior: el último aviso (durante `MESSAGE_DURATION`), las teclas de la
//! búsqueda mientras se escribe o los atajos más útiles de la vista activa.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::chrome::Context;
use super::theme;
use crate::i18n::Phrase;
use crate::tui::action::Action;
use crate::tui::glyphs::Glyphs;
use crate::tui::prompt::PromptKind;
use crate::tui::state::{AppState, Notice, NoticeLevel, ViewKind};

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
        None if searching => {
            // Las teclas de la búsqueda las maneja el prompt, no la tabla de atajos.
            let keys = context.phrase(Phrase::SearchKeys);
            Line::styled(format!("{LEADING_SPACE}{keys}"), theme::MUTED)
        }
        None => hints_line(state.view, context),
    };
    frame
        .buffer_mut()
        .set_line(area.x, area.y, &line, area.width);
}

/// Atajos que sugiere cada vista; la tecla sale de la tabla `BINDINGS`.
fn view_hints(view: ViewKind) -> &'static [(Action, Phrase)] {
    match view {
        ViewKind::Queue => &[
            (Action::Activate, Phrase::HintPlay),
            (Action::Delete, Phrase::HintDelete),
            (Action::ShowPlaying, Phrase::HintShowPlaying),
            (Action::StartSearch, Phrase::HintSearch),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Library => &[
            (Action::Forward, Phrase::HintOpen),
            (Action::Back, Phrase::HintBack),
            (Action::Add, Phrase::HintAdd),
            (Action::AddAll, Phrase::HintAddAlbum),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Browse => &[
            (Action::Activate, Phrase::HintOpenOrAdd),
            (Action::Back, Phrase::HintParent),
            (Action::Add, Phrase::HintAdd),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Search => &[
            (Action::StartSearch, Phrase::HintSearch),
            (Action::Add, Phrase::HintAdd),
            (Action::AddAll, Phrase::HintAddAll),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Playlists => &[
            (Action::Activate, Phrase::HintLoad),
            (Action::Delete, Phrase::HintDelete),
            (Action::SaveQueue, Phrase::HintSaveQueue),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Lyrics => &[
            (Action::TogglePause, Phrase::HintPause),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Visualizer => &[
            (Action::ShowVisualizer, Phrase::HintNextStyle),
            (Action::TogglePause, Phrase::HintPause),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
    }
}

/// En modo compacto solo se sugiere la ayuda.
const COMPACT_HINTS: &[(Action, Phrase)] = &[(Action::ToggleHelp, Phrase::HintHelp)];

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
        spans.push(Span::styled(context.key(*action), theme::ACCENT));
        let label = context.phrase(*label);
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
