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

pub(crate) use self::format::{format_duration, visualizer_style_name};

use self::chrome::{Context, titled_block};
use self::layout::{COMPACT_MIN_HEIGHT, COMPACT_MIN_WIDTH, LayoutMode, layout_mode};
use super::appearance::Appearance;
use super::glyphs::Glyphs;
use super::state::AppState;
use crate::i18n::{Catalog, Message, Phrase};

const FOOTER_ROWS: u16 = 1;
const TABS_ROWS: u16 = 1;
const RULE_ROWS: u16 = 1;
const APPLICATION_NAME: &str = "mpctui";

pub fn render(frame: &mut Frame, state: &mut AppState, appearance: Appearance, now: Instant) {
    let glyphs = appearance.glyphs;
    let area = frame.area();
    let mode = layout_mode(area.width, area.height);
    if mode == LayoutMode::TooSmall {
        return render_too_small(frame, area, glyphs, appearance.texts);
    }
    let context = Context {
        glyphs,
        texts: appearance.texts,
        mode,
        width: area.width,
        now,
        progress_bars: appearance.progress_bars,
        lyrics_line: appearance.lyrics_line,
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
        help::render(frame, area, &context);
    }
}

/// Aviso con el mínimo y el tamaño actual.
fn render_too_small(frame: &mut Frame, area: Rect, glyphs: &Glyphs, texts: &dyn Catalog) {
    let size = texts.message(&Message::TerminalSize {
        minimum_width: COMPACT_MIN_WIDTH,
        minimum_height: COMPACT_MIN_HEIGHT,
        width: area.width,
        height: area.height,
    });
    let lines = vec![
        Line::default(),
        Line::from(format!("  {}", texts.phrase(Phrase::TerminalTooSmall))),
        Line::from(format!("  {size}")),
    ];
    let block = titled_block(Line::from(APPLICATION_NAME), glyphs);
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::time::Duration;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::domain::{
        AudioFormat, BandFrame, ChannelLevel, Entry, LyricLine, Lyrics, LyricsSource, PlayState,
        PlaylistName, SampleFormat, SampleRange, ScopePoint, SearchText, Song, Status,
        VisualContent, VisualFrame, VisualizerStyle, Volume,
    };
    use crate::i18n::{English, Spanish};
    use crate::repository::Error;
    use crate::service::{PlaybackClock, PlayerSnapshot, VisualState};
    use crate::tui::appearance::ProgressBars;
    use crate::tui::glyphs::{ASCII, UNICODE};
    use crate::tui::list::SelectionPolicy;
    use crate::tui::prompt::{Confirmation, Prompt, PromptKind};
    use crate::tui::state::{
        ConnectionState, LyricsStatus, Notice, NoticeLevel, ViewKind, VisualView,
    };

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
    /// Columnas y filas de los cuadros de ejemplo de la Onda y la Cascada.
    const WAVE_COLUMNS: usize = 200;
    const WATERFALL_COLUMNS: usize = 90;
    const WATERFALL_BANDS: usize = 14;
    const SCOPE_POINTS: usize = 600;

    /// Graves altos que bajan hacia los agudos, con picos por encima.
    fn sample_bands() -> BandFrame {
        let levels: Vec<f32> = (0..SPECTRUM_BANDS)
            .map(|band| 1.0 - band as f32 / SPECTRUM_BANDS as f32)
            .collect();
        let peaks = levels
            .iter()
            .map(|level| (level + 0.15).min(0.99))
            .collect();
        BandFrame { levels, peaks }
    }

    /// Un cuadro de ejemplo de cada forma.
    fn sample_content(style: VisualizerStyle) -> VisualContent {
        match style {
            VisualizerStyle::Bars => VisualContent::Bars(sample_bands()),
            VisualizerStyle::Mirror => {
                let mut right = sample_bands();
                right.levels.reverse();
                VisualContent::Mirror {
                    left: sample_bands(),
                    right,
                }
            }
            VisualizerStyle::Wave => VisualContent::Wave {
                columns: (0..WAVE_COLUMNS)
                    .map(|column| {
                        let value = (column as f32 * 0.2).sin() * 0.8;
                        SampleRange {
                            low: value - 0.05,
                            high: value + 0.05,
                        }
                    })
                    .collect(),
                synced: true,
                span: Duration::from_millis(23),
            },
            VisualizerStyle::Waterfall => VisualContent::Waterfall {
                columns: (0..WATERFALL_COLUMNS)
                    .map(|column| {
                        (0..WATERFALL_BANDS)
                            .map(|band| ((column + band) % 10) as f32 / 10.0)
                            .collect()
                    })
                    .collect(),
            },
            VisualizerStyle::Vectorscope => VisualContent::Vectorscope {
                points: (0..SCOPE_POINTS)
                    .map(|index| ScopePoint {
                        side: (index as f32 * 0.37).sin() * 0.3,
                        mid: (index as f32 * 0.11).cos() * 0.8,
                    })
                    .collect(),
                correlation: 0.62,
            },
            VisualizerStyle::Meters => VisualContent::Meters {
                channels: vec![
                    ChannelLevel {
                        rms: -9.4,
                        peak: -3.1,
                    },
                    ChannelLevel {
                        rms: -11.2,
                        peak: -4.6,
                    },
                ],
                correlation: Some(0.62),
            },
        }
    }

    fn sample_frame(style: VisualizerStyle) -> VisualState {
        VisualState::Frame(VisualFrame {
            content: sample_content(style),
            signal: true,
        })
    }

    /// Una línea cada 10 s; el tema de ejemplo va por los 133 s (línea 13).
    fn sample_lyrics(synced: bool) -> Lyrics {
        Lyrics {
            lines: (0..30)
                .map(|index| LyricLine {
                    time: synced.then(|| Duration::from_secs(index * 10)),
                    text: format!("Línea número {index} de la letra"),
                })
                .collect(),
            source: LyricsSource::Cache,
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
        state.visual = VisualView {
            state: sample_frame(VisualizerStyle::Bars),
            source: "/tmp/mpd.fifo".into(),
            format: "44100:16:2".into(),
            ..VisualView::default()
        };
        state.lyrics_enabled = true;
        state.lyrics.status = LyricsStatus::Found(sample_lyrics(true));
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
        size: (u16, u16),
        progress_bars: ProgressBars,
    ) -> Buffer {
        render_in(state, glyphs, size, progress_bars, &Spanish)
    }

    /// Los tests de pantalla usan el español salvo que digan otro idioma.
    fn render_in(
        state: &mut AppState,
        glyphs: &'static Glyphs,
        (width, height): (u16, u16),
        progress_bars: ProgressBars,
        texts: &'static dyn Catalog,
    ) -> Buffer {
        let appearance = Appearance {
            glyphs,
            texts,
            progress_bars,
            visualizer_style: VisualizerStyle::default(),
            lyrics_line: true,
        };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, state, appearance, Instant::now()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// Cada vista; el visualizador, una vez por forma.
    fn every_view() -> Vec<(ViewKind, VisualizerStyle)> {
        let mut views: Vec<_> = ViewKind::ALL
            .into_iter()
            .filter(|view| *view != ViewKind::Visualizer)
            .map(|view| (view, VisualizerStyle::default()))
            .collect();
        views.extend(
            VisualizerStyle::ALL
                .into_iter()
                .map(|style| (ViewKind::Visualizer, style)),
        );
        views
    }

    /// Todas las pantallas: cada vista, con cada superposición y en cada tamaño.
    fn every_screen(glyphs: &'static Glyphs, texts: &'static dyn Catalog) -> Vec<Buffer> {
        let mut screens = Vec::new();
        for (view, style) in every_view() {
            for overlay in overlays() {
                for size in SIZES {
                    let mut state = sample_state();
                    state.view = view;
                    state.visual.style = style;
                    state.visual.state = sample_frame(style);
                    overlay(&mut state);
                    screens.push(render_in(&mut state, glyphs, size, ALL_BARS, texts));
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
        let screens = [
            every_screen(&ASCII, &Spanish),
            every_screen(&ASCII, &English),
        ];
        for screen in screens.iter().flatten() {
            let text = screen_text(screen);
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
        assert_eq!(
            every_screen(&UNICODE, &Spanish).len(),
            every_screen(&ASCII, &Spanish).len()
        );
    }

    #[test]
    fn english_screens_speak_english() {
        let mut state = sample_state();
        let screen = screen_lines(&render_in(
            &mut state, &UNICODE, SIZES[0], ALL_BARS, &English,
        ));
        assert_contains(
            &screen,
            &[
                "[1 Queue]",
                "2 Library",
                "60 songs",
                "random",
                "? help",
                "pos ",
            ],
        );
        assert!(
            !screen.contains("Cola") && !screen.contains("aleatorio"),
            "{screen}"
        );
        state.help_visible = true;
        let help = screen_lines(&render_in(&mut state, &ASCII, SIZES[0], ALL_BARS, &English));
        assert_contains(
            &help,
            &["Help", "Navigation", "Quit", "j Down", "any key closes"],
        );
        state.help_visible = false;
        state.prompt = Some(Prompt::confirm(Confirmation::ClearQueue));
        let dialog = screen_lines(&render_in(&mut state, &ASCII, SIZES[0], ALL_BARS, &English));
        assert_contains(&dialog, &["Clear the queue?", "[y] yes", "[n] no"]);
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

    fn visual_screen(
        glyphs: &'static Glyphs,
        size: (u16, u16),
        style: VisualizerStyle,
        state: VisualState,
    ) -> String {
        let mut sample = sample_state();
        sample.view = ViewKind::Visualizer;
        sample.visual.style = style;
        sample.visual.state = state;
        screen_lines(&render_to_buffer(&mut sample, glyphs, size))
    }

    /// La vista completa de una forma con su cuadro de ejemplo.
    fn style_screen(glyphs: &'static Glyphs, style: VisualizerStyle) -> String {
        visual_screen(glyphs, SIZES[0], style, sample_frame(style))
    }

    fn assert_contains(screen: &str, expected: &[&str]) {
        for text in expected {
            assert!(screen.contains(text), "falta {text:?}:\n{screen}");
        }
    }

    #[test]
    fn bars_draw_levels_peaks_and_axis() {
        let screen = style_screen(&UNICODE, VisualizerStyle::Bars);
        assert_contains(
            &screen,
            &[
                "[6 Visual]",
                "Visualizador: Barras  ● ○ ○ ○ ○ ○  1/6",
                "32 bandas  40 Hz - 16 kHz",
                UNICODE.spectrum_levels[8],
                UNICODE.spectrum_peak,
                " 1k ",
                "16k",
            ],
        );
        let ascii = style_screen(&ASCII, VisualizerStyle::Bars);
        assert_contains(&ascii, &["* . . . . .  1/6", "## ##", " 63 "]);
    }

    #[test]
    fn mirror_draws_each_channel_on_its_side() {
        let screen = style_screen(&UNICODE, VisualizerStyle::Mirror);
        assert_contains(
            &screen,
            &[
                "Visualizador: Espejo estéreo  ○ ● ○ ○ ○ ○  2/6",
                "bandas por canal  L arriba, R abajo",
                "─ 1k ─",
                "██ ██",
            ],
        );
        let lines: Vec<&str> = screen.lines().collect();
        let axis = lines
            .iter()
            .position(|line| line.contains("─ 1k ─"))
            .unwrap();
        assert!(lines[axis - 1].contains("██") && lines[axis + 1].contains("██"));
        assert!(lines[..axis].iter().any(|line| line.starts_with("│ L ")));
        assert!(lines[axis..].iter().any(|line| line.starts_with("│ R ")));
        let ascii = style_screen(&ASCII, VisualizerStyle::Mirror);
        assert_contains(&ascii, &["- 1k -", "## ##"]);
    }

    #[test]
    fn wave_draws_points_with_its_scale() {
        let screen = style_screen(&UNICODE, VisualizerStyle::Wave);
        assert_contains(
            &screen,
            &[
                "Visualizador: Onda",
                "3/6",
                "23 ms  sincronizado",
                "│ +1",
                "│ -1",
            ],
        );
        let braille = |character: char| ('\u{2801}'..='\u{28FF}').contains(&character);
        assert!(screen.chars().any(braille));
        let ascii = style_screen(&ASCII, VisualizerStyle::Wave);
        assert!(!ascii.chars().any(braille));
        assert_contains(&ascii, &["|"]);
    }

    #[test]
    fn waterfall_shades_bands_over_time() {
        let screen = style_screen(&UNICODE, VisualizerStyle::Waterfall);
        assert_contains(
            &screen,
            &[
                "Visualizador: Cascada",
                "4/6",
                "bandas  últimos 9 s",
                "16k",
                " 63",
                "-1 s",
                " ahora │",
                "▓",
            ],
        );
        let ascii = style_screen(&ASCII, VisualizerStyle::Waterfall);
        assert_contains(&ascii, &["%", "#", "-3 s"]);
    }

    #[test]
    fn vectorscope_draws_points_guides_and_correlation() {
        let screen = style_screen(&UNICODE, VisualizerStyle::Vectorscope);
        assert_contains(
            &screen,
            &[
                "Visualizador: Vectorscopio",
                "5/6",
                "M/S  600 muestras",
                "correlación +0.62",
                " M ",
                UNICODE.correlation_center,
            ],
        );
        let ascii = style_screen(&ASCII, VisualizerStyle::Vectorscope);
        assert_contains(&ascii, &["\\", "/", "#", "-1 ", " +1"]);
    }

    #[test]
    fn meters_draw_levels_scale_and_legend() {
        let screen = style_screen(&UNICODE, VisualizerStyle::Meters);
        assert_contains(
            &screen,
            &[
                "Visualizador: Medidores  ○ ○ ○ ○ ○ ●  6/6",
                "RMS y pico por canal",
                "-9.4 dB  pico   -3.1",
                "-11.2 dB  pico   -4.6",
                "┴",
                "dBFS",
                "-60",
                "correlación +0.62",
                "barra = RMS (300 ms)   │ = pico retenido 1,5 s",
                "█",
            ],
        );
        let ascii = style_screen(&ASCII, VisualizerStyle::Meters);
        assert_contains(&ascii, &["####", "+--", "pico retenido 1,5 s"]);
    }

    #[test]
    fn mono_fifos_count_four_styles_and_one_meter() {
        let mut sample = sample_state();
        sample.view = ViewKind::Visualizer;
        sample.visual.channels = 1;
        sample.visual.style = VisualizerStyle::Meters;
        sample.visual.state = VisualState::Frame(VisualFrame {
            content: VisualContent::Meters {
                channels: vec![ChannelLevel {
                    rms: -9.4,
                    peak: -3.1,
                }],
                correlation: None,
            },
            signal: true,
        });
        let screen = screen_lines(&render_to_buffer(&mut sample, &ASCII, SIZES[0]));
        assert_contains(&screen, &[". . . *  4/4", "|  M  "]);
        assert!(!screen.contains("correlación") && !screen.contains("|  R  "));
    }

    #[test]
    fn a_frame_of_another_style_is_not_drawn() {
        let screen = visual_screen(
            &ASCII,
            SIZES[0],
            VisualizerStyle::Wave,
            sample_frame(VisualizerStyle::Bars),
        );
        assert_contains(&screen, &["Visualizador: Onda"]);
        assert!(!screen.contains("bandas") && !screen.contains("## ##"));
    }

    /// Lo que publica cada forma cuando ya no queda nada que dibujar.
    fn flat_content(style: VisualizerStyle) -> VisualContent {
        let zeros = BandFrame {
            levels: vec![0.0; SPECTRUM_BANDS],
            peaks: vec![0.0; SPECTRUM_BANDS],
        };
        match style {
            VisualizerStyle::Bars => VisualContent::Bars(zeros),
            VisualizerStyle::Mirror => VisualContent::Mirror {
                left: zeros.clone(),
                right: zeros,
            },
            VisualizerStyle::Wave => VisualContent::Wave {
                columns: vec![SampleRange::default(); WAVE_COLUMNS],
                synced: false,
                span: Duration::from_millis(23),
            },
            VisualizerStyle::Waterfall => VisualContent::Waterfall {
                columns: vec![vec![0.0; WATERFALL_BANDS]; WATERFALL_COLUMNS],
            },
            VisualizerStyle::Vectorscope => VisualContent::Vectorscope {
                points: Vec::new(),
                correlation: 0.0,
            },
            VisualizerStyle::Meters => VisualContent::Meters {
                channels: vec![ChannelLevel::SILENT; 2],
                correlation: Some(0.0),
            },
        }
    }

    #[test]
    fn visual_view_without_signal_says_so() {
        for style in VisualizerStyle::ALL {
            let silent = VisualState::Frame(VisualFrame {
                content: flat_content(style),
                signal: false,
            });
            let screen = visual_screen(&ASCII, SIZES[0], style, silent);
            assert_contains(&screen, &["sin señal"]);
        }
    }

    #[test]
    fn visual_view_explains_a_missing_fifo() {
        let missing =
            VisualState::Unavailable(Error::from(io::Error::from(io::ErrorKind::NotFound)));
        let screen = visual_screen(&ASCII, SIZES[0], VisualizerStyle::Bars, missing);
        assert_contains(
            &screen,
            &[
                "No se puede leer /tmp/mpd.fifo (no existe).",
                "format \"44100:16:2\"",
                "visualizer_fifo y visualizer_format",
            ],
        );
        assert!(screen.lines().any(|line| line.ends_with("sin señal |")));
        assert!(
            screen
                .lines()
                .any(|line| line.starts_with("| Visualizador "))
        );
    }

    #[test]
    fn compact_styles_hide_titles_and_axes() {
        let compact = |style| visual_screen(&UNICODE, SIZES[1], style, sample_frame(style));
        let bars = compact(VisualizerStyle::Bars);
        assert!(bars.contains(UNICODE.spectrum_levels[8]));
        assert!(!bars.contains("bandas") && !bars.contains("16k"));
        let mirror = compact(VisualizerStyle::Mirror);
        assert!(mirror.contains("────") && !mirror.contains(" 1k "));
        assert!(!compact(VisualizerStyle::Wave).contains("+1"));
        assert!(!compact(VisualizerStyle::Waterfall).contains("ahora"));
        assert!(!compact(VisualizerStyle::Vectorscope).contains("correlación"));
        let meters = compact(VisualizerStyle::Meters);
        assert!(meters.contains("-9.4 dB") && !meters.contains("dBFS"));
    }

    #[test]
    fn the_view_asks_for_the_resolution_it_draws() {
        let mut sample = sample_state();
        sample.view = ViewKind::Visualizer;
        render_to_buffer(&mut sample, &UNICODE, SIZES[0]);
        let request = sample.visual.request.unwrap();
        assert_eq!(
            (request.style, request.columns),
            (VisualizerStyle::Bars, 32)
        );
        sample.visual.style = VisualizerStyle::Wave;
        render_to_buffer(&mut sample, &UNICODE, SIZES[0]);
        assert_eq!(sample.visual.request.unwrap().columns, 2 * (96 - 3));
        render_to_buffer(&mut sample, &ASCII, SIZES[0]);
        assert_eq!(sample.visual.request.unwrap().columns, 96 - 3);
        sample.visual.style = VisualizerStyle::Waterfall;
        render_to_buffer(&mut sample, &ASCII, SIZES[1]);
        let compact = sample.visual.request.unwrap();
        assert_eq!(compact.columns, 72 - 4);
        assert!(compact.rows > 0);
    }

    fn lyrics_screen(glyphs: &'static Glyphs, size: (u16, u16), status: LyricsStatus) -> String {
        let mut sample = sample_state();
        sample.view = ViewKind::Lyrics;
        sample.lyrics.status = status;
        screen_lines(&render_to_buffer(&mut sample, glyphs, size))
    }

    #[test]
    fn synced_lyrics_center_the_current_line() {
        let screen = lyrics_screen(&UNICODE, SIZES[0], LyricsStatus::Found(sample_lyrics(true)));
        assert_contains(
            &screen,
            &[
                "[7 Letra]",
                "Letra: Un tema con un nombre bastante largo 3 — Pink Floyd",
                "sincronizada · caché",
                "▸ Línea número 13 de la letra",
                "Línea número 12 de la letra",
            ],
        );
        let lines: Vec<&str> = screen.lines().collect();
        let current = lines
            .iter()
            .position(|line| line.contains("número 13"))
            .unwrap();
        assert!(
            lines[current - 1].contains("número 12") && lines[current + 1].contains("número 14")
        );
        assert!(!screen.contains("número 0 "));
    }

    #[test]
    fn plain_lyrics_scroll_from_the_top() {
        let mut sample = sample_state();
        sample.view = ViewKind::Lyrics;
        sample.lyrics.status = LyricsStatus::Found(sample_lyrics(false));
        sample.lyrics.scroll = usize::MAX;
        let screen = screen_lines(&render_to_buffer(&mut sample, &ASCII, SIZES[0]));
        assert_contains(&screen, &["texto · caché", "Línea número 29 de la letra"]);
        assert!(!screen.contains("> Línea"));
        assert!(
            sample.lyrics.scroll < 30,
            "el desplazamiento se limita al dibujar"
        );
    }

    #[test]
    fn lyrics_states_are_explained() {
        let searching = lyrics_screen(&ASCII, SIZES[0], LyricsStatus::Searching);
        assert_contains(&searching, &["buscando letra |"]);
        let instrumental = lyrics_screen(&ASCII, SIZES[0], LyricsStatus::Instrumental);
        assert_contains(&instrumental, &["Instrumental"]);
        let none = lyrics_screen(&ASCII, SIZES[0], LyricsStatus::NoSong);
        assert_contains(&none, &["Sin tema en reproducción"]);
        let untitled = LyricsStatus::NotFound {
            untitled: true,
            problem: None,
        };
        let untitled = lyrics_screen(&ASCII, SIZES[0], untitled);
        assert_contains(
            &untitled,
            &[
                "No se encontró la letra para «Un tema con un nombre bastante largo 3»",
                "faltan el artista o el título",
            ],
        );
        assert!(
            !untitled.contains("reintentar"),
            "sin error no hay qué reintentar"
        );
        let failed = LyricsStatus::NotFound {
            untitled: false,
            problem: Some(Error::Closed),
        };
        assert_contains(
            &lyrics_screen(&ASCII, SIZES[0], failed),
            &[
                "Último error: la conexión con MPD se cerró",
                "Ctrl+r reintentar",
            ],
        );
    }

    #[test]
    fn compact_lyrics_hide_the_title() {
        let screen = lyrics_screen(&UNICODE, SIZES[1], LyricsStatus::Found(sample_lyrics(true)));
        assert_contains(&screen, &["▸ Línea número 13"]);
        assert!(!screen.contains("sincronizada"));
    }

    #[test]
    fn disabled_lyrics_have_no_tab() {
        let mut sample = sample_state();
        sample.lyrics_enabled = false;
        let screen = screen_lines(&render_to_buffer(&mut sample, &UNICODE, SIZES[0]));
        assert!(screen.contains("6 Visual") && !screen.contains("7 Letra"));
    }

    #[test]
    fn long_names_are_truncated_with_the_ellipsis() {
        let mut state = sample_state();
        state.view = ViewKind::Library;
        let text = screen_text(&render_to_buffer(&mut state, &ASCII, SIZES[0]));
        assert!(text.contains(ASCII.ellipsis));
    }
}
