//! Playlists (`5`): nombres a la izquierda y contenido de la seleccionada a la derecha.
//! La columna con foco lleva el título entre corchetes y la selección invertida.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use super::{GroupedLook, draw_grouped, draw_line, draw_list};
use crate::domain::PlaylistName;
use crate::i18n::{Catalog, Message, Phrase};
use crate::tui::action::Action;
use crate::tui::glyphs::Glyphs;
use crate::tui::state::{AppState, GroupedSongs, PlaylistsColumn};
use crate::tui::views::chrome::{self, Context};
use crate::tui::views::format::format_duration;
use crate::tui::views::theme;
use crate::tui::views::widgets::{ItemRows, RowSource, SongColumns, centered_message, panel_title};

const NAMES_SHARE: u16 = 30;
/// Separación entre paneles: la ocupa el divisor.
const DIVIDER_WIDTH: u16 = 1;
/// Separación entre una insignia y el nombre.
const BADGE_GAP: &str = " ";

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let playlists = &mut state.playlists;
    if playlists.names.items().is_empty() {
        let key = context.key(Action::SaveQueue);
        let text = context.message(Message::NoPlaylists { key: &key });
        return centered_message(frame, area, Line::styled(text, theme::MUTED));
    }
    let [titles_area, rule_area, lists_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [names_column, songs_column] =
        Layout::horizontal([Constraint::Percentage(NAMES_SHARE), Constraint::Fill(1)])
            .spacing(DIVIDER_WIDTH)
            .areas(area);
    chrome::rule(frame.buffer_mut(), rule_area, context.glyphs, None);
    let names_focused = playlists.focus == PlaylistsColumn::Names;
    let names_title = panel_title(
        format!(
            "{} ({})",
            context.phrase(Phrase::ViewPlaylists),
            playlists.names.items().len()
        ),
        names_focused,
    );
    draw_line(
        frame,
        column_row(names_column, titles_area),
        &Line::from(names_title),
    );
    let preview_title = preview_title(
        playlists.names.selected(),
        &playlists.preview,
        !names_focused,
        context.texts,
    );
    draw_line(frame, column_row(songs_column, titles_area), &preview_title);

    let glyphs = context.glyphs;
    draw_list(
        frame,
        column_row(names_column, lists_area),
        context,
        &mut playlists.names,
        names_focused,
        |names| name_rows(names, glyphs),
    );
    let look = GroupedLook {
        columns: SongColumns {
            position: false,
            album: context.shows_album(),
        },
        now_playing: None,
        highlight: None,
        selectable: !names_focused,
        header: None,
    };
    draw_grouped(
        frame,
        column_row(songs_column, lists_area),
        context,
        &mut playlists.preview,
        look,
    );
    let divider_rows = area.y..area.bottom();
    chrome::divider(
        frame.buffer_mut(),
        names_column.right(),
        divider_rows,
        glyphs,
        true,
    );
}

/// La parte de una columna que cae en una franja horizontal.
fn column_row(column: Rect, band: Rect) -> Rect {
    Rect::new(column.x, band.y, column.width, band.height)
}

/// `domingo: 5 temas  25:12`.
fn preview_title(
    name: Option<&PlaylistName>,
    preview: &GroupedSongs,
    focused: bool,
    texts: &dyn Catalog,
) -> Line<'static> {
    let Some(name) = name else {
        return Line::default();
    };
    let songs = texts.message(&Message::SongCount(preview.items().len()));
    let mut text = format!("{name}: {songs}");
    if let Some(total) = preview.timeline().total {
        text.push_str(&format!("  {}", format_duration(total)));
    }
    Line::from(panel_title(text, focused))
}

fn name_rows<'a>(names: &'a [PlaylistName], glyphs: &'a Glyphs) -> impl RowSource + 'a {
    ItemRows {
        items: names,
        line: move |name: &'a PlaylistName| {
            Line::from(vec![
                Span::styled(glyphs.playlist, theme::MUTED),
                Span::raw(BADGE_GAP),
                Span::raw(name.as_str()),
            ])
        },
    }
}
