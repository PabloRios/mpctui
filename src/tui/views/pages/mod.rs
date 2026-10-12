//! Contenido de cada vista, dentro del marco (entre la regla del encabezado y la del
//! panel "ahora suena"). Cada vista dice qué filas tiene; dibujarlas es compartido.

mod browse;
mod library;
mod lyrics;
mod outputs;
mod playlists;
mod queue;
mod search;
mod visual;

use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::text::Line;

use super::chrome::Context;
use super::widgets::{
    GroupedSongRows, NowPlaying, RowSource, Selection, SongColumns, content_offset, content_width,
    render_list,
};
use crate::tui::list::SelectableList;
use crate::tui::state::{AppState, GroupedSongs, ViewKind};

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    match state.view {
        ViewKind::Queue => queue::render(frame, area, state, context),
        ViewKind::Library => library::render(frame, area, state, context),
        ViewKind::Browse => browse::render(frame, area, state, context),
        ViewKind::Search => search::render(frame, area, state, context),
        ViewKind::Playlists => playlists::render(frame, area, state, context),
        ViewKind::Visualizer => visual::render(frame, area, state, context),
        ViewKind::Lyrics => lyrics::render(frame, area, state, context),
        ViewKind::Outputs => outputs::render(frame, area, state, context),
    }
}

/// Dibuja una lista simple (una fila por elemento) conservando su desplazamiento.
fn draw_list<'a, T, S: RowSource>(
    frame: &mut Frame,
    area: Rect,
    context: &Context<'_>,
    list: &'a mut SelectableList<T>,
    focused: bool,
    make_rows: impl FnOnce(&'a [T]) -> S,
) {
    let centered = list.take_centering_request();
    let (items, table) = list.parts_mut();
    let rows = make_rows(items);
    let selection = Selection {
        row: table.selected(),
        focused,
        centered,
    };
    let mut offset = table.offset();
    render_list(frame, area, context.glyphs, &rows, selection, &mut offset);
    *table.offset_mut() = offset;
}

/// Cómo se muestra una lista de temas agrupada.
struct GroupedLook<'a> {
    columns: SongColumns,
    now_playing: Option<NowPlaying>,
    highlight: Option<&'a str>,
    /// Con selección visible; sin ella cuando se navega otra columna (contenido de una
    /// playlist con el foco en los nombres).
    selectable: bool,
    /// Fila para la cabecera de columnas, si la vista la muestra.
    header: Option<Rect>,
}

fn draw_grouped(
    frame: &mut Frame,
    area: Rect,
    context: &Context<'_>,
    songs: &mut GroupedSongs,
    look: GroupedLook<'_>,
) {
    let (items, timeline, table, centered) = songs.render_parts();
    let rows = GroupedSongRows {
        songs: items,
        timeline,
        glyphs: context.glyphs,
        texts: context.texts,
        columns: look.columns,
        now_playing: look.now_playing,
        highlight: look.highlight,
    };
    if let Some(header) = look.header {
        let width = content_width(header.width, context.glyphs);
        let line = rows.column_header(width);
        let x = header.x + content_offset(context.glyphs);
        frame.buffer_mut().set_line(x, header.y, &line, width);
    }
    let selected = table
        .selected()
        .filter(|_| look.selectable)
        .and_then(|index| rows.row_of_song(index));
    let selection = Selection {
        row: selected,
        focused: look.selectable,
        centered,
    };
    let mut offset = table.offset();
    render_list(frame, area, context.glyphs, &rows, selection, &mut offset);
    *table.offset_mut() = offset;
}

/// Une segmentos con el glifo de migas de pan.
fn breadcrumbs<'a>(parts: impl IntoIterator<Item = &'a str>, context: &Context<'_>) -> String {
    parts
        .into_iter()
        .collect::<Vec<_>>()
        .join(context.glyphs.breadcrumb)
}

/// Margen a cada lado de las filas de texto (migas de pan, búsqueda, resúmenes).
const TEXT_PADDING: u16 = 1;

/// Línea de una sola fila, dibujada sin salirse del área.
fn draw_line(frame: &mut Frame, area: Rect, line: &Line<'_>) {
    frame
        .buffer_mut()
        .set_line(area.x, area.y, line, area.width);
}

/// Fila de texto con margen a los costados.
fn text_row(area: Rect) -> Rect {
    area.inner(Margin::new(TEXT_PADDING, 0))
}
