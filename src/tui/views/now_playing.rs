//! Panel "ahora suena": tema (con la línea actual de la letra, si se configuró), barras
//! del tema, del álbum y de la cola, volumen y modos.
//! Completo (5 filas) o compacto (3 filas).

use std::time::Duration;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::text::{Line, Span};

use super::chrome::{self, Context, block_title, end_title};
use super::format::{
    ARTIST_ALBUM_SEPARATOR, AUDIO_PART_SEPARATOR, AudioPart, AudioPartKind, AudioQuality, MISSING,
    TIME_UNKNOWN, audio_parts, duration_or_unknown, fit_line, fit_parts_from_left, format_duration,
};
use super::gauges::{
    ALBUM_BAR, BAR_MIN_WIDTH, BarInput, BarPolicy, MeterFill, QUEUE_BAR, SONG_BAR, bar_line, meter,
    meter_line, segmented_bar, unknown_bar_line,
};
use super::layout::LayoutMode;
use super::widgets::{millis, spread, state_glyph};
use crate::domain::{AlbumRun, MAX_VOLUME, PlayState, SingleMode, Song, Status, Timeline};
use crate::i18n::{Catalog, Phrase};
use crate::tui::action::Action;
use crate::tui::appearance::ProgressBars;
use crate::tui::glyphs::Glyphs;
use crate::tui::state::{AppState, LyricsStatus};
use crate::tui::theme::{GradientRole, Role};

/// Tema, barra del tema y controles; las barras del álbum y de la cola son opcionales.
pub const FULL_BASE_ROWS: u16 = 3;
pub const COMPACT_ROWS: u16 = 3;
/// Margen a cada lado del contenido del panel.
const SIDE_PADDING: u16 = 1;
/// Separación mínima entre lo que va a la izquierda y a la derecha de una fila.
const MIN_GAP: usize = 2;
const LABEL_GAP: usize = 2;
const VOLUME_CELLS: usize = 10;
const VOLUME_GAP: &str = "    ";
const MODE_GAP: &str = "  ";
/// Entre `tema 4/10` y los tiempos, en las barras del álbum y de la cola.
const ALBUM_DETAIL_GAP: &str = "   ";
const QUEUE_DETAIL_GAP: &str = "  ";
/// Peso de un tema en el modo por cantidad (cuando falta alguna duración).
const COUNT_MODE_WEIGHT: u64 = 1_000;
/// Entre el título y la línea actual de la letra.
const LYRIC_SEPARATOR: &str = " · ";

pub fn rows(mode: LayoutMode, bars: ProgressBars) -> u16 {
    match mode {
        LayoutMode::Compact => COMPACT_ROWS,
        LayoutMode::Full | LayoutMode::TooSmall => FULL_BASE_ROWS + bars.count(),
    }
}

/// Regla que abre el panel: `├─ ▶ reproduciendo ──── FLAC  44.1 kHz  16 bit … ─┤`.
/// En modo compacto, sin títulos.
pub fn render_rule(buffer: &mut Buffer, row: Rect, state: &AppState, context: &Context<'_>) {
    if context.is_compact() {
        return chrome::rule(buffer, row, context, None);
    }
    let title = title(state, context);
    let used = block_title(title.clone(), context).width()
        + MIN_GAP
        + end_title(Line::default(), context).width();
    chrome::rule(buffer, row, context, Some(title));
    let available = usize::from(row.width).saturating_sub(used);
    if let Some(audio) = audio_format(state, available, context) {
        chrome::rule_end_title(buffer, row, context, audio);
    }
}

/// Formato de audio del tema actual en `width` columnas, coloreado por calidad.
/// Detenido, nada: MPD no informa `audio` ni `bitrate`.
fn audio_format(state: &AppState, width: usize, context: &Context<'_>) -> Option<Line<'static>> {
    let status = &state.player.status;
    if status.state == PlayState::Stop {
        return None;
    }
    let parts = audio_parts(
        state.player.current_song.as_ref(),
        status.audio.as_ref(),
        status.bitrate,
        context.texts,
    );
    let fitting = fit_parts_from_left(&parts, width)?;
    let mut spans = Vec::new();
    for (index, part) in fitting.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(AUDIO_PART_SEPARATOR));
        }
        spans.push(Span::styled(part.text.clone(), audio_style(part, context)));
    }
    Some(Line::from(spans))
}

/// El códec según su calidad; la frecuencia y los bits, si son de alta resolución.
fn audio_style(part: &AudioPart, context: &Context<'_>) -> ratatui::style::Style {
    let role = match part.kind {
        AudioPartKind::Codec(AudioQuality::Lossless) => Role::Lossless,
        AudioPartKind::Codec(AudioQuality::Lossy) => Role::Lossy,
        AudioPartKind::Sample {
            high_resolution: true,
        } => Role::HighResolution,
        AudioPartKind::Codec(AudioQuality::Unknown)
        | AudioPartKind::Sample { .. }
        | AudioPartKind::Channels
        | AudioPartKind::Bitrate => Role::Muted,
    };
    context.style(role)
}

/// El rol del estado de reproducción: `playing`, `paused` o, detenido, `muted`.
fn play_state_role(play_state: PlayState) -> Role {
    match play_state {
        PlayState::Play => Role::Playing,
        PlayState::Pause => Role::Paused,
        PlayState::Stop => Role::Muted,
    }
}

/// Título del bloque: `▶ reproduciendo`, `⏸ en pausa`, `■ detenido`.
fn title(state: &AppState, context: &Context<'_>) -> Line<'static> {
    let play_state = state.player.status.state;
    let word = context.phrase(match play_state {
        PlayState::Play => Phrase::Playing,
        PlayState::Pause => Phrase::Paused,
        PlayState::Stop => Phrase::Stopped,
    });
    let glyph = state_glyph(play_state, context.glyphs);
    Line::styled(
        format!("{glyph} {word}"),
        context.style(play_state_role(play_state)),
    )
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let area = area.inner(Margin::new(SIDE_PADDING, 0));
    let playback = Playback::from_state(state, context);
    let lines = if context.is_compact() {
        compact_lines(&playback, area.width, context)
    } else {
        full_lines(&playback, area.width, context)
    };
    let rows = Layout::vertical(vec![Constraint::Length(1); lines.len()]).split(area);
    for (line, row) in lines.into_iter().zip(rows.iter()) {
        frame.buffer_mut().set_line(row.x, row.y, &line, row.width);
    }
}

/// Lo que el panel necesita saber de la reproducción, leído una vez por dibujado.
struct Playback<'a> {
    status: &'a Status,
    song: Option<&'a Song>,
    /// Índice del tema actual en la cola, si la cola lo tiene.
    index: Option<usize>,
    elapsed: Duration,
    duration: Option<Duration>,
    queue: &'a [Song],
    timeline: &'a Timeline,
    /// Línea de la letra que se canta ahora, si el panel la muestra.
    lyric: Option<&'a str>,
}

impl<'a> Playback<'a> {
    fn from_state(state: &'a AppState, context: &Context<'_>) -> Self {
        let queue = state.queue.items();
        let index = state
            .player
            .status
            .song_position
            .and_then(|position| usize::try_from(position).ok())
            .filter(|index| *index < queue.len());
        Self {
            status: &state.player.status,
            song: state.player.current_song.as_ref(),
            index,
            elapsed: state.clock.elapsed_at(context.now),
            duration: state.clock.duration(),
            queue,
            timeline: state.queue.timeline(),
            lyric: current_lyric(state, context),
        }
    }

    fn stopped(&self) -> bool {
        self.status.state == PlayState::Stop
    }

    /// Posición de la cabeza; detenido, no hay cabeza.
    fn head(&self, position: u64) -> Option<u64> {
        (!self.stopped()).then_some(position)
    }

    fn elapsed_text(&self, elapsed: Option<Duration>) -> String {
        if self.stopped() {
            TIME_UNKNOWN.to_owned()
        } else {
            duration_or_unknown(elapsed)
        }
    }

    /// Avance dentro del tema en unidades del modo por cantidad.
    fn counted_progress(&self) -> u64 {
        let Some(duration) = self.duration.filter(|duration| !duration.is_zero()) else {
            return 0;
        };
        let progress = u128::from(millis(self.elapsed)) * u128::from(COUNT_MODE_WEIGHT)
            / u128::from(millis(duration));
        u64::try_from(progress)
            .unwrap_or(COUNT_MODE_WEIGHT)
            .min(COUNT_MODE_WEIGHT - 1)
    }
}

/// La línea de la letra sincronizada que empezó y todavía no terminó, si el panel la
/// muestra. Detenido, antes de la primera línea o en una línea vacía, nada.
fn current_lyric<'a>(state: &'a AppState, context: &Context<'_>) -> Option<&'a str> {
    if !context.lyrics_line || state.player.status.state == PlayState::Stop {
        return None;
    }
    let LyricsStatus::Found(lyrics) = &state.lyrics.status else {
        return None;
    };
    let index = lyrics.current_line(state.clock.elapsed_at(context.now))?;
    let text = lyrics.lines.get(index)?.text.trim();
    (!text.is_empty()).then_some(text)
}

/// `♪ título` y, si hay, ` · línea de la letra`, con cada parte ya recortada.
fn title_spans(
    title: Line<'static>,
    lyric: Option<Line<'static>>,
    context: &Context<'_>,
) -> Vec<Span<'static>> {
    let prefix = now_playing_prefix(context.glyphs);
    let mut spans = vec![Span::styled(prefix, context.style(Role::Accent))];
    spans.extend(title.spans);
    if let Some(lyric) = lyric {
        spans.push(Span::styled(LYRIC_SEPARATOR, context.style(Role::Muted)));
        spans.extend(lyric.spans);
    }
    spans
}

fn now_playing_prefix(glyphs: &Glyphs) -> String {
    format!("{} ", glyphs.now_playing)
}

/// Contenido de una barra: segmentos, o desconocida (stream).
enum BarContent {
    Segments(BarInput, BarPolicy),
    Unknown,
}

struct BarRow {
    label: &'static str,
    content: BarContent,
    /// `tema 4/10   9:52 / 42:51`: lo que cuenta en `muted`, los tiempos en `text`.
    right: Line<'static>,
}

/// `detalle` + separación + `tiempos`; cualquiera de los dos puede faltar.
fn bar_right(
    detail: Option<String>,
    gap: &'static str,
    times: Option<String>,
    context: &Context<'_>,
) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(detail) = detail {
        spans.push(Span::styled(detail, context.style(Role::Muted)));
    }
    if let Some(times) = times {
        if !spans.is_empty() {
            spans.push(Span::raw(gap));
        }
        spans.push(Span::styled(times, context.style(Role::Text)));
    }
    Line::from(spans)
}

const EMPTY_BAR: BarInput = BarInput {
    weights: Vec::new(),
    position: None,
};

fn song_bar(playback: &Playback<'_>, context: &Context<'_>) -> BarRow {
    let times = format!(
        "{} / {}",
        playback.elapsed_text(Some(playback.elapsed)),
        duration_or_unknown(playback.duration)
    );
    let right = bar_right(None, "", Some(times), context);
    let content = match playback.duration {
        Some(duration) => {
            let input = BarInput {
                weights: vec![millis(duration)],
                position: playback.head(millis(playback.elapsed)),
            };
            BarContent::Segments(input, SONG_BAR)
        }
        None if playback.stopped() || playback.song.is_none() => {
            BarContent::Segments(EMPTY_BAR, SONG_BAR)
        }
        None => BarContent::Unknown,
    };
    BarRow {
        label: context.phrase(Phrase::SongLabel),
        content,
        right,
    }
}

/// El tema actual y el tramo (álbum) de la cola que lo contiene.
fn current_album(playback: &Playback<'_>) -> Option<(usize, AlbumRun)> {
    let index = playback.index?;
    let run = playback.timeline.runs[playback.timeline.run_of(index)?];
    Some((index, run))
}

/// `tema 4/10`.
fn song_number(index: usize, run: AlbumRun, texts: &dyn Catalog) -> String {
    format!(
        "{} {}/{}",
        texts.phrase(Phrase::SongLabel),
        index - run.first_index + 1,
        run.song_count
    )
}

/// Un segmento por tema del álbum en curso. Si falta alguna duración, por cantidad.
fn album_bar(playback: &Playback<'_>, context: &Context<'_>) -> BarRow {
    let label = context.phrase(Phrase::AlbumLabel);
    let Some((index, run)) = current_album(playback) else {
        return BarRow {
            label,
            content: BarContent::Segments(EMPTY_BAR, ALBUM_BAR),
            right: Line::default(),
        };
    };
    let in_album = index - run.first_index;
    let songs = &playback.queue[run.indices()];
    let number = song_number(index, run, context.texts);
    let (input, right) = match run.duration {
        Some(total) => {
            let weights: Vec<u64> = songs
                .iter()
                .map(|song| song.duration.map(millis).unwrap_or_default())
                .collect();
            let before: u64 = weights[..in_album].iter().sum();
            let elapsed = Duration::from_millis(before) + playback.elapsed;
            let times = format!(
                "{} / {}",
                playback.elapsed_text(Some(elapsed)),
                format_duration(total)
            );
            let right = bar_right(Some(number), ALBUM_DETAIL_GAP, Some(times), context);
            let position = playback.head(before + millis(playback.elapsed));
            (BarInput { weights, position }, right)
        }
        None => {
            let position = in_album as u64 * COUNT_MODE_WEIGHT + playback.counted_progress();
            let input = BarInput {
                weights: vec![COUNT_MODE_WEIGHT; run.song_count],
                position: playback.head(position),
            };
            (input, bar_right(Some(number), "", None, context))
        }
    };
    BarRow {
        label,
        content: BarContent::Segments(input, ALBUM_BAR),
        right,
    }
}

/// Un segmento por álbum de la cola, con ventana alrededor del actual.
fn queue_bar(playback: &Playback<'_>, context: &Context<'_>) -> BarRow {
    let label = context.phrase(Phrase::QueueLabel);
    let timeline = playback.timeline;
    let current = playback
        .index
        .and_then(|index| Some((index, timeline.run_of(index)?)));
    let Some((index, run)) = current else {
        return BarRow {
            label,
            content: BarContent::Segments(EMPTY_BAR, QUEUE_BAR),
            right: Line::default(),
        };
    };
    let album = context.phrase(Phrase::AlbumLabel);
    let number = format!("{album} {}/{}", run + 1, timeline.runs.len());
    let offset = timeline.song_offsets[index];
    let (input, right) = match (timeline.total, offset) {
        (Some(total), Some(offset)) => {
            let weights = timeline
                .runs
                .iter()
                .map(|run| run.duration.map(millis).unwrap_or_default())
                .collect();
            let elapsed = offset + playback.elapsed;
            let times = format!(
                "{} / {}",
                playback.elapsed_text(Some(elapsed)),
                format_duration(total)
            );
            let input = BarInput {
                weights,
                position: playback.head(millis(elapsed)),
            };
            let right = bar_right(Some(number), QUEUE_DETAIL_GAP, Some(times), context);
            (input, right)
        }
        _ => {
            let weights = timeline
                .runs
                .iter()
                .map(|run| run.song_count as u64 * COUNT_MODE_WEIGHT)
                .collect();
            let position = index as u64 * COUNT_MODE_WEIGHT + playback.counted_progress();
            let input = BarInput {
                weights,
                position: playback.head(position),
            };
            (input, bar_right(Some(number), "", None, context))
        }
    };
    BarRow {
        label,
        content: BarContent::Segments(input, QUEUE_BAR),
        right,
    }
}

/// Barra en `width` columnas, contando sus extremos. Muy angosta, no se dibuja. La de un
/// stream lleva `live` en el medio.
fn draw_bar(content: &BarContent, width: usize, context: &Context<'_>) -> Line<'static> {
    let glyphs = context.glyphs;
    let ends = Span::raw(glyphs.bar_open).width() + Span::raw(glyphs.bar_close).width();
    let inner = width.saturating_sub(ends);
    if inner < BAR_MIN_WIDTH {
        return Line::raw(" ".repeat(width));
    }
    match content {
        BarContent::Segments(input, policy) => {
            let more_width = Span::raw(glyphs.bar_more).width();
            bar_line(&segmented_bar(input, inner, *policy, more_width), context)
        }
        BarContent::Unknown => {
            let live = context.phrase(Phrase::Live);
            unknown_bar_line(inner, &format!(" {live} "), context)
        }
    }
}

/// Las barras empiezan en la misma columna y tienen el mismo ancho.
fn bar_lines(rows: Vec<BarRow>, width: u16, context: &Context<'_>) -> Vec<Line<'static>> {
    let label_width = rows
        .iter()
        .map(|row| Span::raw(row.label).width())
        .max()
        .unwrap_or_default()
        + LABEL_GAP;
    let right_width = rows
        .iter()
        .map(|row| row.right.width())
        .max()
        .unwrap_or_default();
    let bar_width = usize::from(width).saturating_sub(label_width + MIN_GAP + right_width);
    rows.into_iter()
        .map(|row| {
            let label = format!("{:<label_width$}", row.label);
            let mut spans = vec![Span::styled(label, context.style(Role::Muted))];
            spans.extend(draw_bar(&row.content, bar_width, context).spans);
            let padding = " ".repeat(right_width.saturating_sub(row.right.width()));
            let mut right = vec![Span::raw(padding)];
            right.extend(row.right.spans);
            spread(Line::from(spans), Line::from(right), width, context.glyphs)
        })
        .map(owned)
        .collect()
}

fn full_lines(playback: &Playback<'_>, width: u16, context: &Context<'_>) -> Vec<Line<'static>> {
    let bars = context.progress_bars;
    let mut lines = vec![song_line(playback, width, context)];
    let mut rows = vec![song_bar(playback, context)];
    if bars.album {
        rows.push(album_bar(playback, context));
    }
    if bars.queue {
        rows.push(queue_bar(playback, context));
    }
    lines.extend(bar_lines(rows, width, context));
    lines.push(controls_line(playback, width, context));
    lines
}

/// `♪ título · línea de la letra` a la izquierda y `artista - álbum (año)` a la
/// derecha. Si no entra, se trunca primero el álbum, después la letra, el artista y
/// por último el título.
fn song_line(playback: &Playback<'_>, width: u16, context: &Context<'_>) -> Line<'static> {
    let glyphs = context.glyphs;
    let prefix = now_playing_prefix(glyphs);
    let Some(song) = playback.song else {
        let nothing = context.phrase(Phrase::NothingPlaying);
        return Line::styled(format!("{prefix}{nothing}"), context.style(Role::Muted));
    };
    let title = song.display_title().to_owned();
    let artist = song
        .artist
        .clone()
        .or_else(|| song.album_artist.clone())
        .unwrap_or_default();
    let album = song
        .album
        .as_deref()
        .map_or_else(String::new, |album| match song.year() {
            Some(year) => format!("{album} ({year})"),
            None => album.to_owned(),
        });
    let separator = if artist.is_empty() || album.is_empty() {
        ""
    } else {
        ARTIST_ALBUM_SEPARATOR
    };
    let lyric_separator = if playback.lyric.is_some() {
        LYRIC_SEPARATOR
    } else {
        ""
    };
    let lyric = playback.lyric.unwrap_or_default().to_owned();
    let measure = |text: &str| Span::raw(text).width();
    let fixed = measure(&prefix) + MIN_GAP + measure(separator) + measure(lyric_separator);
    let budget = usize::from(width).saturating_sub(fixed);
    let [album_width, lyric_width, artist_width, title_width] = shrink_in_order(
        [
            measure(&album),
            measure(&lyric),
            measure(&artist),
            measure(&title),
        ],
        budget,
        measure(glyphs.ellipsis),
    );
    let fit = |text: String, width: usize, style| {
        owned(fit_line(Line::styled(text, style), width, glyphs.ellipsis))
    };
    let lyric = playback
        .lyric
        .is_some()
        .then(|| fit(lyric, lyric_width, context.style(Role::Muted)));
    let title = fit(title, title_width, context.style(Role::TextStrong));
    let left = title_spans(title, lyric, context);
    let mut right = fit(artist, artist_width, context.style(Role::Artist)).spans;
    right.push(Span::styled(separator, context.style(Role::Muted)));
    right.extend(fit(album, album_width, context.style(Role::Album)).spans);
    owned(spread(Line::from(left), Line::from(right), width, glyphs))
}

/// Recorta anchos en orden de prioridad, cada uno hasta `minimum`, hasta que entren.
fn shrink_in_order<const N: usize>(
    widths: [usize; N],
    budget: usize,
    minimum: usize,
) -> [usize; N] {
    let mut widths = widths;
    let mut overflow = widths.iter().sum::<usize>().saturating_sub(budget);
    for width in &mut widths {
        let cut = overflow.min(width.saturating_sub(minimum));
        *width -= cut;
        overflow -= cut;
    }
    widths
}

/// Un modo del reproductor, con la tecla que lo cambia.
struct Mode {
    action: Action,
    label: &'static str,
    flag: Flag,
    active: bool,
}

/// El indicador de un modo: activo, inactivo o una vez (single).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    On,
    Off,
    Once,
}

impl Flag {
    fn of(active: bool) -> Self {
        if active { Flag::On } else { Flag::Off }
    }

    fn glyph(self, glyphs: &Glyphs) -> &'static str {
        match self {
            Flag::On => glyphs.flag_on,
            Flag::Off => glyphs.flag_off,
            Flag::Once => glyphs.flag_once,
        }
    }

    fn role(self) -> Role {
        match self {
            Flag::On => Role::Success,
            Flag::Off => Role::Subtle,
            Flag::Once => Role::Warning,
        }
    }
}

fn modes(status: &Status, context: &Context<'_>) -> [Mode; 4] {
    let flag = Flag::of;
    let single = match status.single {
        SingleMode::Off => Flag::Off,
        SingleMode::On => Flag::On,
        SingleMode::Oneshot => Flag::Once,
    };
    [
        Mode {
            action: Action::ToggleRandom,
            label: context.phrase(Phrase::ModeRandom),
            flag: flag(status.random),
            active: status.random,
        },
        Mode {
            action: Action::ToggleRepeat,
            label: context.phrase(Phrase::ModeRepeat),
            flag: flag(status.repeat),
            active: status.repeat,
        },
        Mode {
            action: Action::CycleSingle,
            label: context.phrase(Phrase::ModeSingle),
            flag: single,
            active: status.single != SingleMode::Off,
        },
        Mode {
            action: Action::ToggleConsume,
            label: context.phrase(Phrase::ModeConsume),
            flag: flag(status.consume),
            active: status.consume,
        },
    ]
}

/// `z ● aleatorio` (o `z[x] aleatorio`); compacto, `z●` (o `z[x]`). El nombre va en
/// `text` si el modo está activo y en `muted` si no.
fn mode_spans(mode: &Mode, context: &Context<'_>, compact: bool) -> Vec<Span<'static>> {
    let glyphs = context.glyphs;
    let key = context.key(mode.action);
    let gap = if compact { "" } else { glyphs.flag_key_gap };
    let flag = mode.flag.glyph(glyphs);
    let mut spans = vec![
        Span::styled(key, context.strong(Role::Accent)),
        Span::raw(gap),
        Span::styled(
            format!("{}{flag}{}", glyphs.bar_open, glyphs.bar_close),
            context.style(mode.flag.role()),
        ),
    ];
    if !compact {
        let name = if mode.active { Role::Text } else { Role::Muted };
        spans.push(Span::styled(
            format!(" {}", mode.label),
            context.style(name),
        ));
    }
    spans
}

fn modes_spans(status: &Status, context: &Context<'_>, compact: bool) -> Vec<Span<'static>> {
    let gap = if compact { " " } else { MODE_GAP };
    let mut spans = Vec::new();
    for (index, mode) in modes(status, context).iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(gap));
        }
        spans.extend(mode_spans(mode, context, compact));
    }
    spans
}

fn volume_percent(status: &Status, texts: &dyn Catalog) -> String {
    status.volume.map_or_else(
        || texts.phrase(Phrase::VolumeUnavailable).to_owned(),
        |volume| format!("{}%", volume.value()),
    )
}

/// `pos 304/1229`.
fn queue_position(playback: &Playback<'_>) -> String {
    let current = playback
        .index
        .map_or_else(|| MISSING.to_owned(), |index| (index + 1).to_string());
    format!("{current}/{}", playback.status.queue_length)
}

/// `vol ■■■■■■■□□□  70%     z ● aleatorio ...     pos 304/1229`.
fn controls_line(playback: &Playback<'_>, width: u16, context: &Context<'_>) -> Line<'static> {
    let glyphs = context.glyphs;
    let status = playback.status;
    let level = status.volume.map_or(0, |volume| u64::from(volume.value()));
    let maximum = u64::from(MAX_VOLUME);
    let volume_label = context.phrase(Phrase::VolumeLabel);
    let mut left = vec![Span::styled(
        format!("{volume_label} "),
        context.style(Role::Muted),
    )];
    let volume = MeterFill::Gradient(GradientRole::Volume);
    left.extend(meter_line(&meter(level, maximum, VOLUME_CELLS), volume, context).spans);
    left.push(Span::raw(format!(
        "  {:>4}",
        volume_percent(status, context.texts)
    )));
    left.push(Span::raw(VOLUME_GAP));
    left.extend(modes_spans(status, context, false));
    let position_label = context.phrase(Phrase::PositionLabel);
    let right = Line::styled(
        format!("{position_label} {}", queue_position(playback)),
        context.style(Role::Muted),
    );
    owned(spread(Line::from(left), right, width, glyphs))
}

/// Compacto: barra del tema; tema y números; modos con letras.
fn compact_lines(playback: &Playback<'_>, width: u16, context: &Context<'_>) -> Vec<Line<'static>> {
    let glyphs = context.glyphs;
    let texts = context.texts;
    let state = format!("{} ", state_glyph(playback.status.state, glyphs));
    let times = format!(
        " {}/{}",
        playback.elapsed_text(Some(playback.elapsed)),
        duration_or_unknown(playback.duration)
    );
    let bar_width =
        usize::from(width).saturating_sub(Span::raw(&state).width() + Span::raw(&times).width());
    let state_style = context.style(play_state_role(playback.status.state));
    let mut bar = vec![Span::styled(state, state_style)];
    bar.extend(draw_bar(&song_bar(playback, context).content, bar_width, context).spans);
    bar.push(Span::styled(times, context.style(Role::Text)));

    let song = compact_song_line(playback, context);
    let album_number = current_album(playback)
        .map(|(index, run)| song_number(index, run, texts))
        .unwrap_or_default();
    let numbers = format!(
        "{} {}  {album_number}  {}",
        context.phrase(Phrase::VolumeLabel),
        volume_percent(playback.status, texts),
        queue_position(playback)
    );
    let numbers = Line::styled(numbers, context.style(Role::Muted));
    let details = spread(song, numbers, width, glyphs);
    vec![
        Line::from(bar),
        owned(details),
        Line::from(modes_spans(playback.status, context, true)),
    ]
}

/// `♪ título - artista`; con una línea de la letra, `♪ título · línea` (la letra
/// reemplaza al artista).
fn compact_song_line(playback: &Playback<'_>, context: &Context<'_>) -> Line<'static> {
    let prefix = now_playing_prefix(context.glyphs);
    let Some(song) = playback.song else {
        let nothing = context.phrase(Phrase::NothingPlaying);
        return Line::styled(format!("{prefix}{nothing}"), context.style(Role::Muted));
    };
    let title = Line::styled(
        song.display_title().to_owned(),
        context.style(Role::TextStrong),
    );
    if let Some(lyric) = playback.lyric {
        let lyric = Line::styled(lyric.to_owned(), context.style(Role::Muted));
        return Line::from(title_spans(title, Some(lyric), context));
    }
    let artist = song.artist.clone().unwrap_or_default();
    let separator = if artist.is_empty() {
        ""
    } else {
        ARTIST_ALBUM_SEPARATOR
    };
    let mut spans = title_spans(title, None, context);
    spans.push(Span::styled(separator, context.style(Role::Muted)));
    spans.push(Span::styled(artist, context.style(Role::Artist)));
    Line::from(spans)
}

/// Copia los textos para que la línea no dependa de textos armados en el momento.
fn owned(line: Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .into_iter()
            .map(|span| Span::styled(span.content.into_owned(), span.style))
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::domain::{AudioFormat, LyricLine, Lyrics, LyricsSource, SampleFormat};
    use crate::service::PlaybackClock;
    use crate::tui::glyphs::ASCII;
    use crate::tui::views::chrome::testing;

    fn lyrics_context(mode: LayoutMode, lyrics_line: bool, now: Instant) -> Context<'static> {
        Context {
            mode,
            now,
            lyrics_line,
            ..testing::context(&ASCII)
        }
    }

    /// En pausa a los 15 s de Time, con la letra sincronizada: suena «Ticking away».
    fn singing(lyrics: Lyrics, now: Instant) -> AppState {
        let mut state = AppState::default();
        let status = Status {
            state: PlayState::Pause,
            elapsed: Some(Duration::from_secs(15)),
            duration: Some(Duration::from_secs(400)),
            ..Status::default()
        };
        state.clock = PlaybackClock::new(&status, now);
        state.player.status = status;
        state.player.current_song = Some(Song {
            file: "time.flac".into(),
            title: Some("Time".into()),
            artist: Some("Pink Floyd".into()),
            album: Some("The Dark Side of the Moon".into()),
            ..Song::default()
        });
        state.lyrics.status = LyricsStatus::Found(lyrics);
        state
    }

    fn lyrics(synced: bool) -> Lyrics {
        let line = |seconds, text: &str| LyricLine {
            time: synced.then(|| Duration::from_secs(seconds)),
            text: text.into(),
        };
        Lyrics {
            lines: vec![
                line(0, ""),
                line(10, "Ticking away the moments"),
                line(20, "That make up a dull day"),
            ],
            source: LyricsSource::Cache,
        }
    }

    fn song_text(state: &AppState, context: &Context<'_>, width: u16) -> String {
        let playback = Playback::from_state(state, context);
        if context.is_compact() {
            compact_song_line(&playback, context).to_string()
        } else {
            song_line(&playback, width, context).to_string()
        }
    }

    #[test]
    fn the_current_lyric_follows_the_title() {
        let now = Instant::now();
        let state = singing(lyrics(true), now);
        let full = lyrics_context(LayoutMode::Full, true, now);
        let text = song_text(&state, &full, 100);
        assert!(
            text.starts_with("~ Time · Ticking away the moments "),
            "{text}"
        );
        assert!(
            text.ends_with("Pink Floyd - The Dark Side of the Moon"),
            "{text}"
        );
        let compact = lyrics_context(LayoutMode::Compact, true, now);
        let text = song_text(&state, &compact, 100);
        assert_eq!(text, "~ Time · Ticking away the moments");
    }

    #[test]
    fn without_a_current_lyric_the_song_line_is_unchanged() {
        let now = Instant::now();
        let hidden = lyrics_context(LayoutMode::Full, false, now);
        let shown = lyrics_context(LayoutMode::Full, true, now);
        let plain = singing(lyrics(false), now);
        let expected = song_text(&plain, &hidden, 100);
        assert!(!expected.contains("Ticking"), "{expected}");
        assert_eq!(
            song_text(&singing(lyrics(true), now), &hidden, 100),
            expected
        );
        assert_eq!(song_text(&plain, &shown, 100), expected);
        let mut intro = singing(lyrics(true), now);
        intro.clock = PlaybackClock::new(
            &Status {
                elapsed: Some(Duration::from_secs(5)),
                ..intro.player.status.clone()
            },
            now,
        );
        assert_eq!(song_text(&intro, &shown, 100), expected);
        let mut stopped = singing(lyrics(true), now);
        stopped.player.status.state = PlayState::Stop;
        assert!(!song_text(&stopped, &shown, 100).contains("Ticking"));
    }

    #[test]
    fn the_lyric_is_cut_after_the_album_and_before_the_artist() {
        let now = Instant::now();
        let state = singing(lyrics(true), now);
        let full = lyrics_context(LayoutMode::Full, true, now);
        let text = song_text(&state, &full, 50);
        assert_eq!(text.chars().count(), 50, "{text}");
        assert!(text.starts_with("~ Time · Ticking"), "{text}");
        assert!(text.ends_with("Pink Floyd - ..."), "{text}");
        let narrow = song_text(&state, &full, 30);
        assert!(narrow.contains("Pink Floyd"), "{narrow}");
        assert!(!narrow.contains("Ticking away"), "{narrow}");
    }

    #[test]
    fn shrinking_follows_priority() {
        assert_eq!(shrink_in_order([10, 10, 10], 30, 1), [10, 10, 10]);
        assert_eq!(shrink_in_order([10, 10, 10], 25, 1), [5, 10, 10]);
        assert_eq!(shrink_in_order([10, 10, 10], 12, 1), [1, 1, 10]);
        assert_eq!(shrink_in_order([10, 10, 10], 2, 1), [1, 1, 1]);
    }

    #[test]
    fn audio_format_fits_the_width_without_the_codec_first() {
        let mut state = AppState::default();
        state.player.status = Status {
            state: PlayState::Play,
            audio: Some(AudioFormat {
                sample_rate: Some(44_100),
                format: SampleFormat::Integer { bits: 16 },
                channels: 2,
            }),
            bitrate: Some(1411),
            ..Status::default()
        };
        state.player.current_song = Some(Song {
            file: "a.flac".into(),
            ..Song::default()
        });
        let context = testing::context(&ASCII);
        let text = |state: &AppState, width| {
            audio_format(state, width, &context).map(|line| line.to_string())
        };
        let full = "FLAC  44.1 kHz  16 bit  estéreo  1411 kbps";
        assert_eq!(text(&state, 60).as_deref(), Some(full));
        assert_eq!(
            text(&state, 40).as_deref(),
            Some("44.1 kHz  16 bit  estéreo  1411 kbps")
        );
        state.player.status.state = PlayState::Pause;
        assert_eq!(text(&state, 60).as_deref(), Some(full));
        state.player.status.state = PlayState::Stop;
        assert_eq!(text(&state, 60), None);
    }

    #[test]
    fn the_codec_is_colored_by_its_quality() {
        let mut state = AppState::default();
        state.player.status = Status {
            state: PlayState::Play,
            audio: Some(AudioFormat {
                sample_rate: Some(96_000),
                format: SampleFormat::Integer { bits: 24 },
                channels: 2,
            }),
            ..Status::default()
        };
        state.player.current_song = Some(Song {
            file: "a.flac".into(),
            ..Song::default()
        });
        let context = testing::context(&ASCII);
        let line = audio_format(&state, 60, &context).unwrap();
        let style_of = |text: &str| {
            line.spans
                .iter()
                .find(|span| span.content == text)
                .map(|span| span.style)
        };
        assert_eq!(style_of("FLAC"), Some(context.style(Role::Lossless)));
        assert_eq!(
            style_of("96 kHz"),
            Some(context.style(Role::HighResolution))
        );
        assert_eq!(style_of("estéreo"), Some(context.style(Role::Muted)));
    }

    #[test]
    fn rows_depend_on_the_mode_and_the_optional_bars() {
        let hidden = ProgressBars::default();
        let album = ProgressBars {
            album: true,
            queue: false,
        };
        assert_eq!(rows(LayoutMode::Full, hidden), FULL_BASE_ROWS);
        assert_eq!(rows(LayoutMode::Full, album), FULL_BASE_ROWS + 1);
        assert_eq!(rows(LayoutMode::Compact, album), COMPACT_ROWS);
    }

    #[test]
    fn bars_share_start_and_width() {
        let rows = [
            BarRow {
                label: "tema",
                content: BarContent::Segments(
                    BarInput {
                        weights: vec![100],
                        position: Some(50),
                    },
                    SONG_BAR,
                ),
                right: Line::from("1:00 / 2:00"),
            },
            BarRow {
                label: "álbum",
                content: BarContent::Segments(
                    BarInput {
                        weights: vec![1, 1],
                        position: Some(1),
                    },
                    ALBUM_BAR,
                ),
                right: Line::from("tema 2/2   3:00 / 4:00"),
            },
            BarRow {
                label: "cola",
                content: BarContent::Unknown,
                right: Line::default(),
            },
        ];
        let lines = bar_lines(rows.into(), 60, &testing::context(&ASCII));
        let texts: Vec<String> = lines.iter().map(ToString::to_string).collect();
        let column_of = |text: &str, wanted: char| text.chars().position(|found| found == wanted);
        for text in &texts {
            assert_eq!(text.chars().count(), 60, "{text}");
            assert_eq!(column_of(text, '['), Some(7), "{text}");
        }
        let closes: Vec<_> = texts.iter().map(|text| column_of(text, ']')).collect();
        assert!(closes.windows(2).all(|pair| pair[0] == pair[1]));
    }
}
