//! Cola (`1`): cabecera de columnas (pos, pista, título, álbum, duración) y temas
//! agrupados por álbum.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;

use super::{GroupedLook, draw_grouped};
use crate::i18n::Message;
use crate::tui::action::Action;
use crate::tui::state::{AppState, ViewKind};
use crate::tui::theme::Role;
use crate::tui::views::chrome::Context;
use crate::tui::views::widgets::{NowPlaying, SongColumns, centered_message};

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    if state.queue.items().is_empty() {
        let key = context.key(Action::ShowView(ViewKind::Library));
        let text = context.message(Message::EmptyQueue { key: &key });
        let text = Line::styled(text, context.style(Role::Muted));
        return centered_message(frame, area, text);
    }
    let header_rows = if context.is_compact() { 0 } else { 1 };
    let [header_area, list_area] =
        Layout::vertical([Constraint::Length(header_rows), Constraint::Fill(1)]).areas(area);
    let status = &state.player.status;
    let now_playing = status.song_position.map(|position| NowPlaying {
        position,
        state: status.state,
    });
    let look = GroupedLook {
        columns: SongColumns {
            position: true,
            album: context.shows_album(),
        },
        now_playing,
        highlight: None,
        selectable: true,
        header: (header_rows > 0).then_some(header_area),
    };
    draw_grouped(frame, list_area, context, &mut state.queue, look);
}
