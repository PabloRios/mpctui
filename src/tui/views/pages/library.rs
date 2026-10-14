//! Biblioteca (`2`): migas de pan y tres columnas Artista → Álbum → Tema.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;

use super::{breadcrumbs, draw_line, draw_list, text_row};
use crate::i18n::{Catalog, Phrase};
use crate::tui::state::{AppState, LibraryColumn};
use crate::tui::theme::Role;
use crate::tui::views::chrome::{self, Context};
use crate::tui::views::format::or_unnamed;
use crate::tui::views::widgets::{ItemRows, LibrarySongRows, RowSource, panel_title};

const ARTISTS_SHARE: u16 = 26;
const ALBUMS_SHARE: u16 = 32;
/// Separación entre columnas: la ocupa el divisor.
const DIVIDER_WIDTH: u16 = 1;

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let [crumbs_area, titles_area, rule_area, lists_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let library = &mut state.library;
    let texts = context.texts;
    let crumbs_area = text_row(crumbs_area);
    let crumbs = breadcrumbs(
        [
            Some(context.phrase(Phrase::ViewLibrary)),
            library
                .artists
                .selected()
                .map(|name| or_unnamed(name, texts)),
            library
                .albums
                .selected()
                .map(|name| or_unnamed(name, texts)),
        ]
        .into_iter()
        .flatten(),
        usize::from(crumbs_area.width),
        context,
    );
    draw_line(frame, crumbs_area, &crumbs);
    chrome::rule(frame.buffer_mut(), rule_area, context, None);

    let columns = Layout::horizontal([
        Constraint::Percentage(ARTISTS_SHARE),
        Constraint::Percentage(ALBUMS_SHARE),
        Constraint::Fill(1),
    ])
    .spacing(DIVIDER_WIDTH)
    .split(area);
    let focus = library.focus;
    let titles = [
        (
            Phrase::TagArtist,
            library.artists.items().len(),
            LibraryColumn::Artists,
        ),
        (
            Phrase::TagAlbum,
            library.albums.items().len(),
            LibraryColumn::Albums,
        ),
        (
            Phrase::ColumnSong,
            library.songs.items().len(),
            LibraryColumn::Songs,
        ),
    ];
    for (column, (name, count, kind)) in columns.iter().zip(titles) {
        let name = context.phrase(name);
        let title = panel_title(format!("{name} ({count})"), focus == kind, context);
        let title_area = Rect::new(column.x, titles_area.y, column.width, 1);
        draw_line(frame, title_area, &Line::from(title));
    }
    let list_area =
        |column: &Rect| Rect::new(column.x, lists_area.y, column.width, lists_area.height);
    draw_list(
        frame,
        list_area(&columns[0]),
        context,
        &mut library.artists,
        focus == LibraryColumn::Artists,
        |names| name_rows(names, texts, context.style(Role::Artist)),
    );
    draw_list(
        frame,
        list_area(&columns[1]),
        context,
        &mut library.albums,
        focus == LibraryColumn::Albums,
        |names| name_rows(names, texts, context.style(Role::Album)),
    );
    let songs_context = *context;
    draw_list(
        frame,
        list_area(&columns[2]),
        context,
        &mut library.songs,
        focus == LibraryColumn::Songs,
        |songs| LibrarySongRows::new(songs, songs_context),
    );
    for column in &columns[..2] {
        let rows = titles_area.y..area.bottom();
        chrome::divider(frame.buffer_mut(), column.right(), rows, context, false);
    }
}

/// Artistas o álbumes, cada columna con el color de su dato.
fn name_rows<'a>(names: &'a [String], texts: &'a dyn Catalog, style: Style) -> impl RowSource + 'a {
    ItemRows {
        items: names,
        line: move |name: &'a String| Line::styled(or_unnamed(name, texts), style),
    }
}
