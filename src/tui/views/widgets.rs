//! Listas dibujadas fila por fila: solo se arman las filas visibles (con 1000+ temas no
//! se copia nada al dibujar). Una sola función maneja cursor, foco, desplazamiento,
//! cabecera fija y barra de desplazamiento; cada lista solo dice cómo es cada fila.

use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use super::format::{
    album_title, cell, duration_or_unknown, fit_line, format_duration, highlight_matches,
    or_missing, saturating_u16, songs_count,
};
use super::gauges::{meter, meter_line, meter_width};
use super::theme;
use crate::domain::{PlayState, Song, Timeline};
use crate::tui::glyphs::Glyphs;

/// Columna de la barra de desplazamiento, a la derecha de toda lista.
const SCROLLBAR_WIDTH: u16 = 1;
/// Separación entre el contenido de una fila y la barra de desplazamiento.
const SCROLLBAR_GAP: u16 = 1;
/// Separación entre el cursor y el contenido de la fila.
const CURSOR_GAP: u16 = 1;
/// Separación entre columnas de una tabla.
const COLUMN_GAP: u16 = 1;
/// `h:mm:ss`.
pub const DURATION_WIDTH: u16 = 7;
const TRACK_WIDTH: u16 = 3;
/// Celdas del medidor de duración relativa en la columna Tema.
const RELATIVE_DURATION_CELLS: usize = 6;
/// Proporción Título / Álbum en las tablas de temas.
const TITLE_SHARE: u16 = 55;
const ALBUM_SHARE: u16 = 45;
/// Cifras mínimas de la columna `pos`.
const POSITION_MIN_DIGITS: usize = 3;
/// Repeticiones de la regla a cada lado de una separadora de álbum.
const GROUP_RULE_EDGE: usize = 2;
/// En una separadora: el espacio después del título y al menos una celda de regla.
const SEPARATOR_MIN_FILL: usize = 2;

/// Una fila visible.
pub enum ListRow<'a> {
    /// Fila seleccionable: el cursor va delante del contenido.
    Item(Line<'a>),
    /// Fila a todo el ancho que no se selecciona (separadora de álbum).
    Full(Line<'a>),
}

/// De dónde salen las filas de una lista.
pub trait RowSource {
    fn row_count(&self) -> usize;
    /// La fila `index`, para un contenido de `width` columnas.
    fn row(&self, index: usize, width: u16) -> ListRow<'_>;
    /// Fila que encabeza a `index` (la separadora de su tramo). Si la primera fila
    /// visible no es su propio encabezado, se dibuja el encabezado en su lugar.
    fn header_of(&self, index: usize) -> usize {
        index
    }
}

/// Estado de selección de una lista al dibujarla.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub row: Option<usize>,
    /// Con foco, la selección va en video inverso; sin foco, solo el cursor.
    pub focused: bool,
    /// Dejar la selección centrada (salto al tema que suena).
    pub centered: bool,
}

/// Ancho disponible para el contenido de una fila `Item`.
pub fn content_width(area_width: u16, glyphs: &Glyphs) -> u16 {
    area_width.saturating_sub(content_offset(glyphs) + SCROLLBAR_GAP + SCROLLBAR_WIDTH)
}

/// Columnas que ocupan el cursor y su separación.
pub fn content_offset(glyphs: &Glyphs) -> u16 {
    saturating_u16(Span::raw(glyphs.cursor).width()) + CURSOR_GAP
}

/// Dibuja las filas visibles y actualiza `offset` (primera fila visible).
pub fn render_list(
    frame: &mut Frame,
    area: Rect,
    glyphs: &Glyphs,
    source: &impl RowSource,
    selection: Selection,
    offset: &mut usize,
) {
    let height = usize::from(area.height);
    let total = source.row_count();
    let top = scroll_offset(source, selection, *offset, height);
    *offset = top;
    let width = content_width(area.width, glyphs);
    let full_width = area.width.saturating_sub(SCROLLBAR_WIDTH);
    for (screen_row, index) in (top..total.min(top + height)).enumerate() {
        let index = if screen_row == 0 {
            source.header_of(index)
        } else {
            index
        };
        let row_area = Rect::new(area.x, area.y + saturating_u16(screen_row), full_width, 1);
        match source.row(index, width) {
            ListRow::Full(line) => {
                let line = fit_line(line, usize::from(full_width), glyphs.ellipsis);
                frame
                    .buffer_mut()
                    .set_line(row_area.x, row_area.y, &line, full_width);
            }
            ListRow::Item(line) => {
                let selected = selection.row == Some(index);
                draw_item(frame, row_area, glyphs, line, selected);
                if selected && selection.focused {
                    frame.buffer_mut().set_style(row_area, theme::SELECTED_ROW);
                }
            }
        }
    }
    render_scrollbar(frame, area, glyphs, total, selection.row.unwrap_or(top));
}

fn draw_item(frame: &mut Frame, row_area: Rect, glyphs: &Glyphs, line: Line<'_>, selected: bool) {
    let buffer = frame.buffer_mut();
    if selected {
        buffer.set_string(row_area.x, row_area.y, glyphs.cursor, theme::ACCENT);
    }
    let offset = content_offset(glyphs);
    let width = row_area.width.saturating_sub(offset + SCROLLBAR_GAP);
    let line = fit_line(line, usize::from(width), glyphs.ellipsis);
    buffer.set_line(row_area.x + offset, row_area.y, &line, width);
}

/// Mantiene la selección visible. La fila de arriba puede quedar tapada por la cabecera
/// fija, así que la selección nunca queda en ella si no es su propio encabezado.
fn scroll_offset(
    source: &impl RowSource,
    selection: Selection,
    offset: usize,
    height: usize,
) -> usize {
    let total = source.row_count();
    let Some(selected) = selection.row.filter(|row| *row < total) else {
        return offset.min(total.saturating_sub(height));
    };
    let mut top = if selection.centered {
        selected.saturating_sub(height / 2)
    } else {
        offset
    };
    let highest_allowed = if source.header_of(selected) < selected {
        selected - 1
    } else {
        selected
    };
    top = top.min(highest_allowed);
    top = top.max((selected + 1).saturating_sub(height));
    top = top.min(total.saturating_sub(height));
    // Si arriba quedaría la cabecera fija de un tramo justo antes de la separadora del
    // siguiente, se baja una fila para no mostrar dos separadoras seguidas.
    let next = top + 1;
    if source.header_of(top) != top && next <= highest_allowed && source.header_of(next) == next {
        top = next;
    }
    top
}

/// Barra de desplazamiento en la última columna; solo si la lista no entra completa.
fn render_scrollbar(frame: &mut Frame, area: Rect, glyphs: &Glyphs, total: usize, position: usize) {
    if total <= usize::from(area.height) {
        return;
    }
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some(glyphs.scroll_up))
        .end_symbol(Some(glyphs.scroll_down))
        .track_symbol(Some(glyphs.scroll_track))
        .thumb_symbol(glyphs.scroll_thumb)
        .style(theme::MUTED);
    let mut state = ScrollbarState::new(total)
        .position(position)
        .viewport_content_length(usize::from(area.height));
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

/// Anchos de columnas para un ancho de contenido dado.
fn column_widths<const N: usize>(width: u16, constraints: [Constraint; N]) -> [u16; N] {
    let areas = Layout::horizontal(constraints)
        .spacing(COLUMN_GAP)
        .split(Rect::new(0, 0, width, 1));
    std::array::from_fn(|index| areas[index].width)
}

/// Une celdas de ancho fijo con la separación entre columnas; las de ancho cero se
/// omiten (columnas que no se muestran).
fn join_cells<'a>(cells: Vec<(u16, Vec<Span<'a>>)>) -> Line<'a> {
    let gap = " ".repeat(usize::from(COLUMN_GAP));
    let mut spans = Vec::new();
    for (width, cell_spans) in cells {
        if width == 0 {
            continue;
        }
        if !spans.is_empty() {
            spans.push(Span::raw(gap.clone()));
        }
        spans.extend(cell_spans);
    }
    Line::from(spans)
}

/// Celda de una tabla: el texto ajustado a su columna.
fn table_cell<'a>(
    line: Line<'a>,
    width: u16,
    alignment: Alignment,
    glyphs: &'a Glyphs,
) -> (u16, Vec<Span<'a>>) {
    (
        width,
        cell(line, usize::from(width), alignment, glyphs.ellipsis),
    )
}

/// Lista de una línea por elemento (artistas, álbumes, playlists). La línea se arma
/// solo para las filas visibles.
pub struct ItemRows<'a, T, F> {
    pub items: &'a [T],
    pub line: F,
}

impl<'a, T, F: Fn(&'a T) -> Line<'a>> RowSource for ItemRows<'a, T, F> {
    fn row_count(&self) -> usize {
        self.items.len()
    }

    fn row(&self, index: usize, _width: u16) -> ListRow<'_> {
        ListRow::Item((self.line)(&self.items[index]))
    }
}

/// Glifo del estado de reproducción.
pub fn state_glyph(state: PlayState, glyphs: &Glyphs) -> &'static str {
    match state {
        PlayState::Play => glyphs.play,
        PlayState::Pause => glyphs.pause,
        PlayState::Stop => glyphs.stop,
    }
}

/// Qué columnas muestra una tabla de temas agrupada.
#[derive(Debug, Clone, Copy)]
pub struct SongColumns {
    /// Posición en la cola (solo en la cola).
    pub position: bool,
    pub album: bool,
}

/// El tema que suena, para marcar su fila.
#[derive(Debug, Clone, Copy)]
pub struct NowPlaying {
    pub position: u32,
    pub state: PlayState,
}

/// Tabla de temas agrupada por álbum: separadora antes de cada tramo, cabecera fija.
/// Las filas visibles son `separadoras + temas`; la selección sigue siendo de temas.
pub struct GroupedSongRows<'a> {
    pub songs: &'a [Song],
    pub timeline: &'a Timeline,
    pub glyphs: &'a Glyphs,
    pub columns: SongColumns,
    pub now_playing: Option<NowPlaying>,
    /// Texto buscado, para resaltarlo.
    pub highlight: Option<&'a str>,
}

/// Una fila de la tabla agrupada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupedRow {
    Separator(usize),
    Song(usize),
}

/// Anchos de las columnas de una tabla de temas.
#[derive(Debug, Clone, Copy)]
struct SongWidths {
    state: u16,
    position: u16,
    track: u16,
    title: u16,
    album: u16,
    duration: u16,
}

impl GroupedSongRows<'_> {
    /// Fila visible del tema `song_index`.
    pub fn row_of_song(&self, song_index: usize) -> Option<usize> {
        Some(song_index + self.timeline.run_of(song_index)? + 1)
    }

    /// La separadora `k` está en la fila `first_index + k`: se busca la última que no
    /// pase de `row`.
    fn locate(&self, row: usize) -> GroupedRow {
        let runs = &self.timeline.runs;
        let (mut low, mut high) = (0, runs.len());
        while low < high {
            let middle = (low + high) / 2;
            if runs[middle].first_index + middle <= row {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        let run = low.saturating_sub(1);
        let header = runs.get(run).map_or(0, |found| found.first_index + run);
        if row == header {
            GroupedRow::Separator(run)
        } else {
            GroupedRow::Song(row - run - 1)
        }
    }

    fn widths(&self, width: u16) -> SongWidths {
        let state = [self.glyphs.play, self.glyphs.pause, self.glyphs.stop]
            .iter()
            .map(|glyph| Span::raw(*glyph).width())
            .max()
            .unwrap_or_default();
        let digits = self.songs.len().to_string().len().max(POSITION_MIN_DIGITS);
        let position = if self.columns.position { digits } else { 0 };
        let album = if self.columns.album { ALBUM_SHARE } else { 0 };
        let [state, position, track, title, album, duration] = column_widths(
            width,
            [
                Constraint::Length(saturating_u16(state)),
                Constraint::Length(saturating_u16(position)),
                Constraint::Length(TRACK_WIDTH),
                Constraint::Fill(TITLE_SHARE),
                Constraint::Fill(album),
                Constraint::Length(DURATION_WIDTH),
            ],
        );
        SongWidths {
            state,
            position,
            track,
            title,
            album,
            duration,
        }
    }

    /// Cabecera de columnas alineada con las filas.
    pub fn column_header(&self, width: u16) -> Line<'static> {
        let widths = self.widths(width);
        let glyphs = self.glyphs;
        let header = |text: &'static str, width: u16, alignment| {
            let (width, spans) = table_cell(Line::from(text), width, alignment, glyphs);
            let spans: Vec<Span<'static>> = spans
                .into_iter()
                .map(|span| Span::styled(span.content.into_owned(), span.style))
                .collect();
            (width, spans)
        };
        join_cells(vec![
            header("", widths.state, Alignment::Left),
            header("pos", widths.position, Alignment::Right),
            header("#", widths.track, Alignment::Right),
            header("Título", widths.title, Alignment::Left),
            header("Álbum", widths.album, Alignment::Left),
            header("dur", widths.duration, Alignment::Right),
        ])
        .style(theme::TABLE_HEADER)
    }

    fn song_line(&self, index: usize, width: u16) -> Line<'_> {
        let song = &self.songs[index];
        let widths = self.widths(width);
        let glyphs = self.glyphs;
        let playing = self
            .now_playing
            .filter(|now| song.position == Some(now.position));
        let style = if playing.is_some() {
            theme::CURRENT_SONG
        } else {
            theme::PLAIN
        };
        let state = playing.map_or("", |now| state_glyph(now.state, glyphs));
        let position = song
            .position
            .map(|position| (u64::from(position) + 1).to_string())
            .unwrap_or_default();
        let track = song.track_number().unwrap_or_default();
        let album = or_missing(song.album.as_deref());
        let duration = song.duration.map(format_duration).unwrap_or_default();
        join_cells(vec![
            table_cell(
                Line::styled(state, style),
                widths.state,
                Alignment::Left,
                glyphs,
            ),
            table_cell(
                Line::styled(position, theme::MUTED),
                widths.position,
                Alignment::Right,
                glyphs,
            ),
            table_cell(
                Line::styled(track, theme::MUTED),
                widths.track,
                Alignment::Right,
                glyphs,
            ),
            table_cell(
                highlight_matches(song.display_title(), self.highlight, style),
                widths.title,
                Alignment::Left,
                glyphs,
            ),
            table_cell(
                highlight_matches(album, self.highlight, style),
                widths.album,
                Alignment::Left,
                glyphs,
            ),
            table_cell(
                Line::styled(duration, style),
                widths.duration,
                Alignment::Right,
                glyphs,
            ),
        ])
    }

    /// `── Artista - Álbum (año) ─────── 10 temas  42:51 ──`. Se trunca solo el título:
    /// la cantidad y la duración nunca.
    fn separator_line(&self, run_index: usize, width: u16) -> Line<'static> {
        let run = self.timeline.runs[run_index];
        let rule = self.glyphs.group_rule;
        let mut summary = songs_count(run.song_count);
        if let Some(duration) = run.duration {
            summary.push_str(&format!("  {}", format_duration(duration)));
        }
        let edge = rule.repeat(GROUP_RULE_EDGE);
        let left = format!("{edge} ");
        let right = format!(" {summary} {edge}");
        let fixed = Span::raw(&left).width() + Span::raw(&right).width() + SEPARATOR_MIN_FILL;
        let title = album_title(&self.songs[run.first_index]);
        let title = owned_line(highlight_matches(
            &title,
            self.highlight,
            theme::GROUP_TITLE,
        ));
        let available = usize::from(width).saturating_sub(fixed);
        let title = fit_line(title, available, self.glyphs.ellipsis);
        // Una celda de regla ya está contada en `fixed`.
        let fill = available.saturating_sub(title.width()) + SEPARATOR_MIN_FILL - 1;
        let mut spans = vec![Span::styled(left, theme::GROUP_RULE)];
        spans.extend(title.spans);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(rule.repeat(fill), theme::GROUP_RULE));
        spans.push(Span::styled(format!(" {summary}"), theme::MUTED));
        spans.push(Span::styled(format!(" {edge}"), theme::GROUP_RULE));
        Line::from(spans)
    }
}

/// Copia los textos de una línea para que no dependa de un texto armado en el momento.
fn owned_line(line: Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .into_iter()
            .map(|span| Span::styled(span.content.into_owned(), span.style))
            .collect::<Vec<_>>(),
    )
}

impl RowSource for GroupedSongRows<'_> {
    fn row_count(&self) -> usize {
        self.songs.len() + self.timeline.runs.len()
    }

    fn row(&self, index: usize, width: u16) -> ListRow<'_> {
        match self.locate(index) {
            GroupedRow::Separator(run) => {
                let full_width = width + content_offset(self.glyphs) + SCROLLBAR_GAP;
                ListRow::Full(self.separator_line(run, full_width))
            }
            GroupedRow::Song(song) => ListRow::Item(self.song_line(song, width)),
        }
    }

    fn header_of(&self, index: usize) -> usize {
        match self.locate(index) {
            GroupedRow::Separator(_) => index,
            GroupedRow::Song(song) => {
                let run = self.timeline.run_of(song).unwrap_or_default();
                self.timeline.runs[run].first_index + run
            }
        }
    }
}

/// Columna Tema de la biblioteca: pista, título, duración y duración relativa al tema
/// más largo del álbum mostrado.
pub struct LibrarySongRows<'a> {
    songs: &'a [Song],
    glyphs: &'a Glyphs,
    longest: Option<Duration>,
}

impl<'a> LibrarySongRows<'a> {
    pub fn new(songs: &'a [Song], glyphs: &'a Glyphs) -> Self {
        let longest = songs.iter().filter_map(|song| song.duration).max();
        Self {
            songs,
            glyphs,
            longest,
        }
    }

    /// Con duración desconocida, la celda queda vacía.
    fn relative_duration(&self, song: &Song) -> Line<'static> {
        match (song.duration, self.longest) {
            (Some(duration), Some(longest)) => {
                let cells = meter(millis(duration), millis(longest), RELATIVE_DURATION_CELLS);
                meter_line(&cells, self.glyphs)
            }
            _ => Line::default(),
        }
    }
}

impl RowSource for LibrarySongRows<'_> {
    fn row_count(&self) -> usize {
        self.songs.len()
    }

    fn row(&self, index: usize, width: u16) -> ListRow<'_> {
        let song = &self.songs[index];
        let glyphs = self.glyphs;
        let meter_columns = saturating_u16(meter_width(RELATIVE_DURATION_CELLS, glyphs));
        let [track, title, duration, relative] = column_widths(
            width,
            [
                Constraint::Length(TRACK_WIDTH),
                Constraint::Fill(1),
                Constraint::Length(DURATION_WIDTH),
                Constraint::Length(meter_columns),
            ],
        );
        let number = song.track_number().unwrap_or_default();
        ListRow::Item(join_cells(vec![
            table_cell(
                Line::styled(number, theme::MUTED),
                track,
                Alignment::Right,
                glyphs,
            ),
            table_cell(
                Line::from(song.display_title()),
                title,
                Alignment::Left,
                glyphs,
            ),
            table_cell(
                Line::from(duration_or_unknown(song.duration)),
                duration,
                Alignment::Right,
                glyphs,
            ),
            table_cell(
                self.relative_duration(song),
                relative,
                Alignment::Left,
                glyphs,
            ),
        ]))
    }
}

/// Milisegundos: la unidad de barras y medidores.
pub fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Una parte a la izquierda y otra alineada a la derecha en `width` columnas. Si no
/// entran, se trunca la izquierda.
pub fn spread<'a>(left: Line<'a>, right: Line<'a>, width: u16, glyphs: &'a Glyphs) -> Line<'a> {
    let right_width = right.width();
    let available = usize::from(width).saturating_sub(right_width + 1);
    let left = fit_line(left, available, glyphs.ellipsis);
    let gap = usize::from(width).saturating_sub(left.width() + right_width);
    let mut spans = left.spans;
    spans.push(Span::raw(" ".repeat(gap)));
    spans.extend(right.spans);
    Line::from(spans)
}

/// Una línea centrada vertical y horizontalmente en el área.
pub fn centered_message(frame: &mut Frame, area: Rect, text: Line<'_>) {
    let middle = Rect::new(area.x, area.y + area.height / 2, area.width, 1);
    frame.render_widget(Paragraph::new(text.centered()), middle);
}

/// Título de una columna o panel: con foco va entre corchetes, en negrita e invertido.
pub fn panel_title(text: String, focused: bool) -> Span<'static> {
    if focused {
        Span::styled(format!("[{text}]"), theme::FOCUSED)
    } else {
        Span::styled(format!(" {text} "), theme::PLAIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::timeline;
    use crate::tui::glyphs::ASCII;

    fn album_song(album: &str) -> Song {
        Song {
            file: format!("{album}.mp3"),
            album: Some(album.into()),
            duration: Some(Duration::from_secs(60)),
            ..Song::default()
        }
    }

    fn grouped<'a>(songs: &'a [Song], timeline: &'a Timeline) -> GroupedSongRows<'a> {
        GroupedSongRows {
            songs,
            timeline,
            glyphs: &ASCII,
            columns: SongColumns {
                position: true,
                album: true,
            },
            now_playing: None,
            highlight: None,
        }
    }

    #[test]
    fn grouped_rows_interleave_separators() {
        let songs = [album_song("A"), album_song("A"), album_song("B")];
        let timeline = timeline(&songs);
        let rows = grouped(&songs, &timeline);
        assert_eq!(rows.row_count(), 5);
        let located: Vec<_> = (0..5).map(|row| rows.locate(row)).collect();
        assert_eq!(
            located,
            [
                GroupedRow::Separator(0),
                GroupedRow::Song(0),
                GroupedRow::Song(1),
                GroupedRow::Separator(1),
                GroupedRow::Song(2),
            ]
        );
        assert_eq!(rows.row_of_song(2), Some(4));
        assert_eq!(rows.header_of(2), 0);
        assert_eq!(rows.header_of(4), 3);
    }

    #[test]
    fn song_rows_show_position_track_title_album_and_duration() {
        let songs = [Song {
            title: Some("Time".into()),
            track: Some("4/10".into()),
            position: Some(303),
            ..album_song("Dark Side")
        }];
        let timeline = timeline(&songs);
        let rows = grouped(&songs, &timeline);
        let header = rows.column_header(60).to_string();
        let words: Vec<_> = header.split_whitespace().collect();
        assert_eq!(words, ["pos", "#", "Título", "Álbum", "dur"]);
        let line = rows.song_line(0, 60).to_string();
        let words: Vec<_> = line.split_whitespace().collect();
        assert_eq!(words, ["304", "4", "Time", "Dark", "Side", "1:00"]);
    }

    #[test]
    fn narrow_song_rows_hide_the_album() {
        let songs = [Song {
            title: Some("Time".into()),
            ..album_song("Dark Side")
        }];
        let timeline = timeline(&songs);
        let rows = GroupedSongRows {
            columns: SongColumns {
                position: true,
                album: false,
            },
            ..grouped(&songs, &timeline)
        };
        assert!(!rows.column_header(60).to_string().contains("Álbum"));
        assert!(!rows.song_line(0, 60).to_string().contains("Dark Side"));
    }

    #[test]
    fn separator_never_truncates_the_summary() {
        let songs = [album_song("Un álbum con un nombre larguísimo")];
        let timeline = timeline(&songs);
        let rows = grouped(&songs, &timeline);
        let line = rows.separator_line(0, 40);
        assert_eq!(line.width(), 40);
        let text = line.to_string();
        assert!(text.ends_with(" 1 tema  1:00 =="), "{text}");
        assert!(text.contains(ASCII.ellipsis));
        let wide = rows.separator_line(0, 80);
        assert_eq!(wide.width(), 80);
    }

    #[test]
    fn selection_stays_below_the_sticky_header() {
        let songs: Vec<Song> = (0..30).map(|_| album_song("A")).collect();
        let timeline = timeline(&songs);
        let rows = grouped(&songs, &timeline);
        let selection = |row| Selection {
            row: Some(row),
            focused: true,
            centered: false,
        };
        assert_eq!(scroll_offset(&rows, selection(10), 15, 5), 9);
        assert_eq!(scroll_offset(&rows, selection(20), 0, 5), 16);
        let centered = Selection {
            centered: true,
            ..selection(20)
        };
        assert_eq!(scroll_offset(&rows, centered, 0, 5), 18);
    }

    #[test]
    fn sticky_header_is_not_followed_by_another_separator() {
        let mut songs = vec![album_song("A"); 3];
        songs.extend(vec![album_song("B"); 3]);
        let timeline = timeline(&songs);
        let rows = grouped(&songs, &timeline);
        // Filas: 0 sep A, 1-3 A, 4 sep B, 5-7 B. Con la fila 3 arriba se vería
        // "sep A, sep B"; se arranca en la separadora de B.
        let selection = Selection {
            row: Some(6),
            focused: true,
            centered: false,
        };
        assert_eq!(scroll_offset(&rows, selection, 3, 3), 4);
    }

    #[test]
    fn spread_aligns_right_and_truncates_left() {
        let line = spread(Line::from("izquierda"), Line::from("der"), 15, &ASCII);
        assert_eq!(line.to_string(), "izquierda   der");
        let line = spread(Line::from("izquierda larga"), Line::from("der"), 12, &ASCII);
        assert_eq!(line.to_string(), "izqui... der");
    }
}
