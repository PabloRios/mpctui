//! Línea inferior: el último aviso (durante `MESSAGE_DURATION`), las teclas de la
//! búsqueda mientras se escribe o los atajos más útiles de la vista activa.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::chrome::Context;
use crate::domain::MoveDirection;
use crate::i18n::Phrase;
use crate::tui::action::Action;
use crate::tui::prompt::PromptKind;
use crate::tui::state::{AppState, Notice, NoticeLevel, OutputsColumn, PlaylistsColumn, ViewKind};
use crate::tui::theme::Role;

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
        Some(notice) => notice_line(notice, context),
        None if searching => {
            // Las teclas de la búsqueda las maneja el prompt, no la tabla de atajos.
            let keys = context.phrase(Phrase::SearchKeys);
            Line::styled(format!("{LEADING_SPACE}{keys}"), context.style(Role::Muted))
        }
        None => hints_line(state, context),
    };
    frame
        .buffer_mut()
        .set_line(area.x, area.y, &line, area.width);
}

/// Atajos que sugiere cada vista (en Playlists y Salidas, según la columna con foco); la tecla
/// sale de la tabla `BINDINGS`.
fn view_hints(state: &AppState) -> &'static [(Action, Phrase)] {
    match state.view {
        ViewKind::Queue => &[
            (Action::Activate, Phrase::HintPlay),
            (Action::Delete, Phrase::HintDelete),
            (Action::MoveSong(MoveDirection::Down), Phrase::HintMoveDown),
            (Action::MoveSong(MoveDirection::Up), Phrase::HintMoveUp),
            (Action::AddToPlaylist, Phrase::HintToPlaylist),
            (Action::ShowPlaying, Phrase::HintShowPlaying),
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
            (Action::AddToPlaylist, Phrase::HintToPlaylist),
            (Action::ToggleHelp, Phrase::HintHelp),
        ],
        ViewKind::Playlists => match state.playlists.focus {
            PlaylistsColumn::Names => &[
                (Action::Activate, Phrase::HintLoad),
                (Action::Forward, Phrase::HintEdit),
                (Action::Delete, Phrase::HintDelete),
                (Action::SaveQueue, Phrase::HintSaveQueue),
                (Action::ToggleHelp, Phrase::HintHelp),
            ],
            PlaylistsColumn::Songs => &[
                (Action::MoveSong(MoveDirection::Down), Phrase::HintMoveDown),
                (Action::MoveSong(MoveDirection::Up), Phrase::HintMoveUp),
                (Action::Delete, Phrase::HintRemove),
                (Action::AddToPlaylist, Phrase::HintToPlaylist),
                (Action::Back, Phrase::HintBack),
                (Action::ToggleHelp, Phrase::HintHelp),
            ],
        },
        ViewKind::Outputs => match state.outputs.focus {
            OutputsColumn::Partitions => &[
                (Action::Activate, Phrase::HintUsePartition),
                (Action::Forward, Phrase::HintOutputs),
                (Action::NewPartition, Phrase::HintNewPartition),
                (Action::Delete, Phrase::HintDelete),
                (Action::ToggleHelp, Phrase::HintHelp),
            ],
            OutputsColumn::Outputs => &[
                (Action::Activate, Phrase::HintToggleOutput),
                (Action::MoveOutput, Phrase::HintMoveOutput),
                (Action::NewPartition, Phrase::HintNewPartition),
                (Action::Back, Phrase::HintBack),
                (Action::ToggleHelp, Phrase::HintHelp),
            ],
        },
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

fn hints_line(state: &AppState, context: &Context<'_>) -> Line<'static> {
    let hints = if context.is_compact() {
        COMPACT_HINTS
    } else {
        view_hints(state)
    };
    let mut spans = vec![Span::raw(LEADING_SPACE)];
    for (index, (action, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(HINT_GAP));
        }
        spans.push(Span::styled(
            context.key(*action),
            context.strong(Role::Accent),
        ));
        let label = context.phrase(*label);
        spans.push(Span::styled(
            format!(" {label}"),
            context.style(Role::Muted),
        ));
    }
    Line::from(spans)
}

/// El prefijo distingue el tipo de aviso sin depender del color.
fn notice_line<'a>(notice: &'a Notice, context: &Context<'_>) -> Line<'a> {
    let glyphs = context.glyphs;
    let (prefix, role) = match notice.level {
        NoticeLevel::Info => (glyphs.message_info, Role::Info),
        NoticeLevel::Error => (glyphs.message_error, Role::Error),
    };
    let style = context.style(role);
    Line::from(vec![
        Span::raw(LEADING_SPACE),
        Span::styled(prefix, style),
        Span::raw(PREFIX_GAP),
        Span::styled(notice.text.as_str(), style),
    ])
}
