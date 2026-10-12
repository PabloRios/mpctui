//! Dibujo de la interfaz. Solo lee el estado; nunca llama a servicios.
//!
//! La pantalla es un marco con cuatro zonas: encabezado (borde con el nombre y la
//! conexión, pestañas y regla), la vista activa, el panel "ahora suena" y la línea
//! inferior, fuera del marco.

mod chrome;
mod dialogs;
mod footer;
mod format;
mod gauges;
mod header;
mod help;
mod layout;
mod now_playing;
mod pages;
mod theme;
mod widgets;

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

pub(crate) use self::format::format_duration;

use self::chrome::{Context, titled_block};
use self::layout::{COMPACT_MIN_HEIGHT, COMPACT_MIN_WIDTH, LayoutMode, layout_mode};
use super::appearance::Appearance;
use super::glyphs::Glyphs;
use super::state::AppState;

const FOOTER_ROWS: u16 = 1;
const TABS_ROWS: u16 = 1;
const RULE_ROWS: u16 = 1;
const APPLICATION_NAME: &str = "mpctui";
/// Bordes izquierdo y derecho del marco.
const FRAME_BORDERS: u16 = 2;

pub fn render(frame: &mut Frame, state: &mut AppState, appearance: Appearance, now: Instant) {
    let glyphs = appearance.glyphs;
    let area = frame.area();
    let mode = layout_mode(area.width, area.height);
    if mode == LayoutMode::TooSmall {
        return render_too_small(frame, area, glyphs);
    }
    let context = Context {
        glyphs,
        mode,
        width: area.width,
        now,
        progress_bars: appearance.progress_bars,
    };
    let [frame_area, footer_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(FOOTER_ROWS)]).areas(area);
    let outer = header::frame_block(state, &context);
    let inner = outer.inner(frame_area);
    frame.render_widget(outer, frame_area);
    let [tabs_area, header_rule, body_area, panel_rule, panel_area] = Layout::vertical([
        Constraint::Length(TABS_ROWS),
        Constraint::Length(RULE_ROWS),
        Constraint::Fill(1),
        Constraint::Length(RULE_ROWS),
        Constraint::Length(now_playing::rows(mode, appearance.progress_bars)),
    ])
    .areas(inner);

    header::render_tabs(frame, tabs_area, state, &context);
    chrome::rule(frame.buffer_mut(), header_rule, glyphs, None);
    now_playing::render_rule(frame.buffer_mut(), panel_rule, state, &context);
    pages::render(frame, body_area, state, &context);
    now_playing::render(frame, panel_area, state, &context);
    footer::render(frame, footer_area, state, &context);
    dialogs::render(frame, area, state, &context);
    if state.help_visible {
        help::render(frame, area, glyphs);
    }
}

/// Bandas que dibuja la vista Espectro en una terminal de este tamaño. La app la usa
/// para pedirle al visualizador la cantidad justa.
pub fn spectrum_bands(terminal: Rect) -> usize {
    let mode = layout_mode(terminal.width, terminal.height);
    let body = terminal.width.saturating_sub(FRAME_BORDERS);
    pages::band_count(body, mode == LayoutMode::Compact)
}

/// Aviso con el mínimo y el tamaño actual.
fn render_too_small(frame: &mut Frame, area: Rect, glyphs: &Glyphs) {
    let lines = vec![
        Line::default(),
        Line::from("  La terminal es muy chica."),
        Line::from(format!(
            "  Mínimo: {COMPACT_MIN_WIDTH} x {COMPACT_MIN_HEIGHT}. Actual: {} x {}.",
            area.width, area.height
        )),
    ];
    let block = titled_block(Line::from(APPLICATION_NAME), glyphs);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::domain::{
        AudioFormat, Entry, PlayState, PlaylistName, SampleFormat, SearchText, Song, SpectrumFrame,
        Status, Volume,
    };
    use crate::service::{PlaybackClock, PlayerSnapshot, SpectrumState};
    use crate::tui::appearance::ProgressBars;
    use crate::tui::glyphs::{ASCII, UNICODE};
    use crate::tui::list::SelectionPolicy;
    use crate::tui::prompt::{Confirmation, Prompt, PromptKind};
    use crate::tui::state::{ConnectionState, Notice, NoticeLevel, SpectrumView, ViewKind};

    /// Tamaños de la sección 7 de `DESIGN.md`: completo, compacto y demasiado chico.
    const SIZES: [(u16, u16); 3] = [(100, 30), (72, 16), (40, 10)];
    /// Más elementos de los que entran, para que aparezca la barra de desplazamiento.
    const LONG_LIST: usize = 60;
    /// Rangos que el modo ASCII no puede dibujar: flechas, símbolos técnicos, cajas,
    /// bloques, figuras geométricas y símbolos varios, más la elipsis.
    const FORBIDDEN_IN_ASCII: [std::ops::RangeInclusive<char>; 2] =
        ['\u{2190}'..='\u{2BFF}', '\u{2026}'..='\u{2026}'];

    fn song(index: usize) -> Song {
        Song {
            file: format!("Rock/Pink Floyd/{index:02} - Un tema con un nombre bastante largo.flac"),
            title: Some(format!("Un tema con un nombre bastante largo {index}")),
            artist: Some("Pink Floyd".into()),
            album: Some("The Piper at the Gates of Dawn".into()),
            duration: Some(Duration::from_secs(400)),
            position: u32::try_from(index).ok(),
            ..Song::default()
        }
    }

    const SPECTRUM_BANDS: usize = 32;

    /// Graves altos que bajan hacia los agudos, con picos por encima.
    fn sample_spectrum() -> SpectrumFrame {
        let levels: Vec<f32> = (0..SPECTRUM_BANDS)
            .map(|band| 1.0 - band as f32 / SPECTRUM_BANDS as f32)
            .collect();
        let peaks = levels
            .iter()
            .map(|level| (level + 0.15).min(0.99))
            .collect();
        SpectrumFrame {
            levels,
            peaks,
            signal: true,
        }
    }

    fn songs() -> Vec<Song> {
        (0..LONG_LIST).map(song).collect()
    }

    fn sample_state() -> AppState {
        let mut state = AppState::new("localhost:6600".into());
        let status = Status {
            volume: Some(Volume::new(70).unwrap()),
            state: PlayState::Pause,
            song_position: Some(3),
            elapsed: Some(Duration::from_secs(133)),
            duration: Some(Duration::from_secs(400)),
            audio: Some(AudioFormat {
                sample_rate: Some(44_100),
                format: SampleFormat::Integer { bits: 16 },
                channels: 2,
            }),
            bitrate: Some(1411),
            ..Status::default()
        };
        state.clock = PlaybackClock::new(&status, Instant::now());
        state.player = PlayerSnapshot {
            status,
            current_song: Some(song(3)),
        };
        state.queue.set_items(songs(), SelectionPolicy::Reset);
        let artists = (0..LONG_LIST)
            .map(|index| format!("Artista con un nombre larguísimo número {index}"))
            .collect();
        state
            .library
            .artists
            .set_items(artists, SelectionPolicy::Reset);
        state.library.albums.set_items(
            vec!["The Piper at the Gates of Dawn".into()],
            SelectionPolicy::Reset,
        );
        state
            .library
            .songs
            .set_items(songs(), SelectionPolicy::Reset);
        let entries = vec![
            Entry::Directory {
                path: "Rock/Scans".into(),
            },
            Entry::File(Box::new(song(1))),
            Entry::Playlist {
                path: "Rock/dsotm.m3u".into(),
            },
        ];
        state
            .browse
            .entries
            .set_items(entries, SelectionPolicy::Reset);
        state.search.text = Some(SearchText::new("time").unwrap());
        state
            .search
            .results
            .set_items(songs(), SelectionPolicy::Reset);
        state.spectrum = SpectrumView {
            state: SpectrumState::Frame(sample_spectrum()),
            source: "/tmp/mpd.fifo".into(),
            format: "44100:16:2".into(),
        };
        let names = vec![PlaylistName::new("domingo").unwrap()];
        state
            .playlists
            .names
            .set_items(names, SelectionPolicy::Reset);
        state
            .playlists
            .preview
            .set_items(songs(), SelectionPolicy::Reset);
        state
    }

    /// Variantes que cambian lo que se superpone a la vista.
    fn overlays() -> Vec<fn(&mut AppState)> {
        vec![
            |_| {},
            |state| state.help_visible = true,
            |state| {
                state.connection = ConnectionState::Waiting {
                    attempt: 4,
                    retry_at: Instant::now() + Duration::from_secs(5),
                    delay: Duration::from_secs(8),
                };
                state.database_update = Some(Instant::now());
                state.notice = Some(Notice {
                    level: NoticeLevel::Error,
                    text: "Sin conexión con MPD".into(),
                    shown_at: Instant::now(),
                });
            },
            |state| state.prompt = Some(Prompt::text(PromptKind::Search, "time".into())),
            |state| state.prompt = Some(Prompt::text(PromptKind::SavePlaylist, "x".into())),
            |state| state.prompt = Some(Prompt::confirm(Confirmation::ClearQueue)),
        ]
    }

    /// Con todas las barras opcionales, para que los tests las cubran.
    const ALL_BARS: ProgressBars = ProgressBars {
        album: true,
        queue: true,
    };

    fn render_to_buffer(state: &mut AppState, glyphs: &'static Glyphs, size: (u16, u16)) -> Buffer {
        render_with_bars(state, glyphs, size, ALL_BARS)
    }

    fn render_with_bars(
        state: &mut AppState,
        glyphs: &'static Glyphs,
        (width, height): (u16, u16),
        progress_bars: ProgressBars,
    ) -> Buffer {
        let appearance = Appearance {
            glyphs,
            progress_bars,
        };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, state, appearance, Instant::now()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// Todas las pantallas: cada vista, con cada superposición y en cada tamaño.
    fn every_screen(glyphs: &'static Glyphs) -> Vec<Buffer> {
        let mut screens = Vec::new();
        for view in ViewKind::ALL {
            for overlay in overlays() {
                for size in SIZES {
                    let mut state = sample_state();
                    state.view = view;
                    overlay(&mut state);
                    screens.push(render_to_buffer(&mut state, glyphs, size));
                }
            }
        }
        screens
    }

    fn screen_text(buffer: &Buffer) -> String {
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn ascii_screens_have_no_unicode_glyphs() {
        for screen in every_screen(&ASCII) {
            let text = screen_text(&screen);
            let forbidden = text.chars().find(|character| {
                FORBIDDEN_IN_ASCII
                    .iter()
                    .any(|range| range.contains(character))
            });
            assert_eq!(forbidden, None, "glifo no ASCII en:\n{text}");
        }
    }

    #[test]
    fn unicode_screens_use_unicode_glyphs() {
        let mut state = sample_state();
        let text = screen_text(&render_to_buffer(&mut state, &UNICODE, SIZES[0]));
        for glyph in [UNICODE.cursor, UNICODE.pause, UNICODE.scroll_thumb] {
            assert!(text.contains(glyph), "falta {glyph:?}");
        }
        assert!(text.contains(UNICODE.border.top_left));
        assert_eq!(every_screen(&UNICODE).len(), every_screen(&ASCII).len());
    }

    #[test]
    fn album_and_queue_bars_only_show_when_enabled() {
        let bar_label = |text: &str, label: &str| {
            text.lines()
                .any(|line| line.starts_with(&format!("| {label} ")))
        };
        let mut state = sample_state();
        let hidden = render_with_bars(&mut state, &ASCII, SIZES[0], ProgressBars::default());
        let hidden = screen_lines(&hidden);
        assert!(bar_label(&hidden, "tema"));
        assert!(!bar_label(&hidden, "álbum") && !bar_label(&hidden, "cola"));
        let shown = screen_lines(&render_to_buffer(&mut state, &ASCII, SIZES[0]));
        assert!(bar_label(&shown, "álbum") && bar_label(&shown, "cola"));
    }

    /// El buffer como texto, una línea por fila.
    fn screen_lines(buffer: &Buffer) -> String {
        let width = usize::from(buffer.area.width);
        let cells: Vec<&str> = buffer.content().iter().map(|cell| cell.symbol()).collect();
        cells
            .chunks(width)
            .map(|row| row.concat())
            .collect::<Vec<_>>()
            .join("\n")
    }

    const AUDIO_FORMAT: &str = "FLAC  44.1 kHz  16 bit  estéreo  1411 kbps";

    /// La fila de la regla que abre el panel "ahora suena".
    fn panel_rule(screen: &str) -> Option<&str> {
        screen.lines().find(|line| line.contains("en pausa"))
    }

    #[test]
    fn audio_format_sits_at_the_end_of_the_panel_rule() {
        let mut state = sample_state();
        let screen = screen_lines(&render_to_buffer(&mut state, &ASCII, SIZES[0]));
        let rule = panel_rule(&screen).expect("falta la regla del panel");
        assert!(rule.ends_with(&format!(" {AUDIO_FORMAT} -+")), "{rule}");
        let screen = screen_lines(&render_to_buffer(&mut state, &UNICODE, SIZES[0]));
        assert!(panel_rule(&screen).unwrap().contains(AUDIO_FORMAT));
    }

    #[test]
    fn audio_format_is_hidden_when_stopped_or_compact() {
        let mut state = sample_state();
        let compact = screen_lines(&render_to_buffer(&mut state, &ASCII, SIZES[1]));
        assert!(!compact.contains("kbps"));
        state.player.status.state = PlayState::Stop;
        let stopped = screen_lines(&render_to_buffer(&mut state, &ASCII, SIZES[0]));
        assert!(!stopped.contains("kbps") && !stopped.contains("FLAC"));
    }

    #[test]
    fn header_fits_the_update_and_the_countdown() {
        let mut state = sample_state();
        state.database_update = Some(Instant::now());
        state.connection = ConnectionState::Waiting {
            attempt: 4,
            retry_at: Instant::now() + Duration::from_secs(5),
            delay: Duration::from_secs(8),
        };
        let screen = screen_lines(&render_to_buffer(&mut state, &ASCII, SIZES[0]));
        let header = screen.lines().next().unwrap();
        assert!(header.starts_with("+- mpctui - | actualizando biblioteca 0:00 "));
        assert!(header.contains(" (x) sin conexión, reintento en 5 s [###-----] -+"));
    }

    fn spectrum_screen(glyphs: &'static Glyphs, size: (u16, u16), state: SpectrumState) -> String {
        let mut sample = sample_state();
        sample.view = ViewKind::Spectrum;
        sample.spectrum.state = state;
        screen_lines(&render_to_buffer(&mut sample, glyphs, size))
    }

    #[test]
    fn spectrum_view_draws_bars_peaks_and_axis() {
        let frame = SpectrumState::Frame(sample_spectrum());
        let screen = spectrum_screen(&UNICODE, SIZES[0], frame.clone());
        assert!(screen.contains("[6 Espectro]"));
        assert!(screen.contains("32 bandas  40 Hz - 16 kHz  /tmp/mpd.fifo"));
        for expected in [
            UNICODE.spectrum_levels[8],
            UNICODE.spectrum_peak,
            " 1k ",
            "16k",
        ] {
            assert!(screen.contains(expected), "falta {expected:?}:\n{screen}");
        }
        let ascii = spectrum_screen(&ASCII, SIZES[0], frame);
        assert!(ascii.contains("## ##") && ascii.contains(" 63 "), "{ascii}");
    }

    #[test]
    fn spectrum_view_without_signal_says_so() {
        let silent = SpectrumState::Frame(SpectrumFrame::silent(SPECTRUM_BANDS));
        let screen = spectrum_screen(&ASCII, SIZES[0], silent);
        assert!(screen.contains("sin señal"));
        assert!(!screen.contains("## ##"));
    }

    #[test]
    fn spectrum_view_explains_a_missing_fifo() {
        let missing = SpectrumState::Unavailable("no existe".into());
        let screen = spectrum_screen(&ASCII, SIZES[0], missing);
        assert!(screen.contains("No se puede leer /tmp/mpd.fifo (no existe)."));
        assert!(screen.contains("format \"44100:16:2\""));
        assert!(screen.contains("visualizer_fifo y visualizer_format"));
        assert!(screen.lines().any(|line| line.ends_with("sin señal |")));
    }

    #[test]
    fn compact_spectrum_hides_the_title_and_axis() {
        let screen = spectrum_screen(&UNICODE, SIZES[1], SpectrumState::Frame(sample_spectrum()));
        assert!(screen.contains(UNICODE.spectrum_levels[8]));
        assert!(!screen.contains("bandas") && !screen.contains("16k"));
    }

    #[test]
    fn spectrum_bands_depend_on_the_terminal() {
        assert_eq!(spectrum_bands(Rect::new(0, 0, 100, 30)), 32);
        assert_eq!(spectrum_bands(Rect::new(0, 0, 72, 16)), 64);
    }

    #[test]
    fn long_names_are_truncated_with_the_ellipsis() {
        let mut state = sample_state();
        state.view = ViewKind::Library;
        let text = screen_text(&render_to_buffer(&mut state, &ASCII, SIZES[0]));
        assert!(text.contains(ASCII.ellipsis));
    }
}
