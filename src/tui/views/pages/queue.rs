//! Cola (`1`): cabecera de columnas y temas agrupados por álbum.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;

use super::{GroupedLook, draw_grouped};
use crate::tui::action::{Action, key_label};
use crate::tui::state::{AppState, ViewKind};
use crate::tui::views::chrome::Context;
use crate::tui::views::theme;
use crate::tui::views::widgets::{NowPlaying, SongColumns, centered_message};

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    if state.queue.items().is_empty() {
        let library =
            key_label(Action::ShowView(ViewKind::Library), context.glyphs).unwrap_or_default();
        let text = format!("La cola está vacía. {library} abre la biblioteca para agregar temas.");
        return centered_message(frame, area, Line::styled(text, theme::MUTED));
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
            artist: context.shows_artist(),
        },
        now_playing,
        highlight: None,
        selectable: true,
        header: (header_rows > 0).then_some(header_area),
    };
    draw_grouped(frame, list_area, context, &mut state.queue, look);
}
