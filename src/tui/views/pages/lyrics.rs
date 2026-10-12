//! Letra (`7`): la del tema que suena. Una sincronizada mantiene la línea actual
//! centrada (las ya cantadas, apagadas); una sin sincronizar se recorre con las teclas
//! de navegación. Mientras busca o si no hay letra, lo dice en el centro.

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::{draw_line, text_row};
use crate::domain::{Lyrics, LyricsSource};
use crate::i18n::{Message, Phrase};
use crate::service::PlaybackClock;
use crate::tui::glyphs::Glyphs;
use crate::tui::state::{AppState, LyricsStatus};
use crate::tui::views::chrome::Context;
use crate::tui::views::format::{fit_line, or_missing, saturating_u16};
use crate::tui::views::theme;
use crate::tui::views::widgets::{centered_message, spread};

const TITLE_ROWS: u16 = 1;
/// Entre el título del tema y el artista.
const TITLE_SEPARATOR: &str = " — ";
const DETAIL_SEPARATOR: &str = " · ";
const LRCLIB_NAME: &str = "LRCLIB";
/// Entre el cursor de la línea actual y su texto.
const CURSOR_GAP: &str = " ";

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let title_rows = if context.is_compact() { 0 } else { TITLE_ROWS };
    let [title, body] =
        Layout::vertical([Constraint::Length(title_rows), Constraint::Fill(1)]).areas(area);
    if !title.is_empty() {
        draw_title(frame, text_row(title), state, context);
    }
    let body = text_row(body);
    let glyphs = context.glyphs;
    match &state.lyrics.status {
        LyricsStatus::NoSong => message(frame, body, context.phrase(Phrase::NoSong)),
        LyricsStatus::Instrumental => message(frame, body, context.phrase(Phrase::Instrumental)),
        LyricsStatus::Searching => {
            let spinner = glyphs.spinner_frame(state.ticks);
            let searching = context.phrase(Phrase::SearchingLyrics);
            message(frame, body, &format!("{searching} {spinner}"));
        }
        LyricsStatus::NotFound { untitled, problem } => {
            let detail = if *untitled {
                Some(context.phrase(Phrase::UntitledHint).to_owned())
            } else {
                problem.as_ref().map(|problem| {
                    let label = context.phrase(Phrase::LastError);
                    format!("{label}: {}", context.message(Message::Failure(problem)))
                })
            };
            let title = song_title(state);
            let not_found = context.message(Message::LyricsNotFound { title: &title });
            draw_not_found(frame, body, not_found, detail);
        }
        LyricsStatus::Found(lyrics) if lyrics.is_synced() => {
            draw_synced(frame, body, lyrics, &state.clock, context.now, glyphs);
        }
        LyricsStatus::Found(lyrics) => {
            let scroll = &mut state.lyrics.scroll;
            *scroll = (*scroll).min(lyrics.lines.len().saturating_sub(usize::from(body.height)));
            draw_plain(frame, body, lyrics, *scroll, glyphs);
        }
    }
}

/// `Letra: Time — Pink Floyd` y, a la derecha, `sincronizada · caché`.
fn draw_title(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let title = context.phrase(Phrase::ViewLyrics);
    let mut left = title.to_owned();
    if let Some(song) = &state.player.current_song {
        let artist = or_missing(song.artist.as_deref().or(song.album_artist.as_deref()));
        left = format!("{title}: {}{TITLE_SEPARATOR}{artist}", song.display_title());
    }
    let details = match &state.lyrics.status {
        LyricsStatus::Found(lyrics) => {
            let timing = context.phrase(if lyrics.is_synced() {
                Phrase::Synced
            } else {
                Phrase::PlainText
            });
            let source = source_name(lyrics.source, context);
            format!("{timing}{DETAIL_SEPARATOR}{source}")
        }
        _ => String::new(),
    };
    let line = spread(
        Line::styled(left, theme::BOLD),
        Line::styled(details, theme::MUTED),
        area.width,
        context.glyphs,
    );
    draw_line(frame, area, &line);
}

/// LRCLIB es un nombre propio: no se traduce.
fn source_name(source: LyricsSource, context: &Context<'_>) -> &'static str {
    match source {
        LyricsSource::Cache => context.phrase(Phrase::SourceCache),
        LyricsSource::SongFolder => context.phrase(Phrase::SourceSongFolder),
        LyricsSource::Tags => context.phrase(Phrase::SourceTags),
        LyricsSource::Lrclib => LRCLIB_NAME,
    }
}

fn song_title(state: &AppState) -> String {
    state
        .player
        .current_song
        .as_ref()
        .map(|song| song.display_title().to_owned())
        .unwrap_or_default()
}

fn message(frame: &mut Frame, area: Rect, text: &str) {
    centered_message(frame, area, Line::styled(text.to_owned(), theme::MUTED));
}

/// `No se encontró la letra para «Time»` y, debajo, el motivo si se sabe.
fn draw_not_found(frame: &mut Frame, area: Rect, not_found: String, detail: Option<String>) {
    if area.is_empty() {
        return;
    }
    let middle = area.y + area.height / 2;
    let rows = [Some(not_found), detail];
    for (offset, text) in rows.into_iter().enumerate() {
        let y = middle + saturating_u16(offset);
        if let Some(text) = text
            && y < area.bottom()
        {
            let row = Rect::new(area.x, y, area.width, 1);
            frame.render_widget(Line::styled(text, theme::MUTED).centered(), row);
        }
    }
}

/// La línea actual en el medio; antes de la primera, la primera espera en el medio.
fn draw_synced(
    frame: &mut Frame,
    area: Rect,
    lyrics: &Lyrics,
    clock: &PlaybackClock,
    now: Instant,
    glyphs: &Glyphs,
) {
    let current = lyrics.current_line(clock.elapsed_at(now));
    let anchor = current.unwrap_or_default();
    let middle = usize::from(area.height / 2);
    for row in 0..usize::from(area.height) {
        let Some(index) = (anchor + row).checked_sub(middle) else {
            continue;
        };
        let Some(line) = lyrics.lines.get(index) else {
            break;
        };
        let style = match current {
            Some(current) if index < current => theme::MUTED,
            Some(current) if index == current => theme::CURRENT_LYRIC,
            _ => theme::PLAIN,
        };
        let is_current = current == Some(index);
        draw_centered(frame, area, row, &line.text, style, is_current, glyphs);
    }
}

fn draw_plain(frame: &mut Frame, area: Rect, lyrics: &Lyrics, scroll: usize, glyphs: &Glyphs) {
    let visible = lyrics
        .lines
        .iter()
        .skip(scroll)
        .take(usize::from(area.height));
    for (row, line) in visible.enumerate() {
        draw_centered(frame, area, row, &line.text, theme::PLAIN, false, glyphs);
    }
}

/// Una línea centrada en la fila `row`; la actual lleva el cursor a la izquierda.
fn draw_centered(
    frame: &mut Frame,
    area: Rect,
    row: usize,
    text: &str,
    style: Style,
    is_current: bool,
    glyphs: &Glyphs,
) {
    let cursor_width = Span::raw(format!("{}{CURSOR_GAP}", glyphs.cursor)).width();
    let available = usize::from(area.width).saturating_sub(2 * cursor_width);
    let line = fit_line(
        Line::styled(text.to_owned(), style),
        available,
        glyphs.ellipsis,
    );
    let width = line.width();
    let x = area.x + saturating_u16((usize::from(area.width) - width) / 2);
    let y = area.y + saturating_u16(row);
    let buffer = frame.buffer_mut();
    buffer.set_line(x, y, &line, saturating_u16(width));
    if is_current {
        let cursor_x = x.saturating_sub(saturating_u16(cursor_width));
        buffer.set_string(cursor_x, y, glyphs.cursor, theme::ACCENT);
    }
}
