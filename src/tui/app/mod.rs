//! Controlador de la TUI: recibe eventos, llama a los servicios y actualiza el estado.
//! Cada vista tiene su propio submódulo con sus acciones.

mod browse;
mod library;
mod lyrics;
mod playback;
mod playlists;
mod prompt;
mod queue;
mod refresh;
mod search;
mod visual;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyEvent, KeyEventKind};
use futures::{Stream, StreamExt};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::UnboundedReceiver;

use super::action::{Action, action_for};
use super::appearance::Appearance;
use super::prompt::{Confirmation, Prompt, PromptKind};
use super::state::{AppState, ConnectionState, Notice, NoticeLevel, ViewKind};
use super::views;
use crate::i18n::{Message, Phrase};
use crate::repository::{
    Error, LibraryRepository, OptionsRepository, PcmSource, PlayerRepository, PlaylistRepository,
    QueueRepository, Result,
};
use crate::service::{
    LibraryService, LyricsLookup, PlaybackService, PlaylistService, QueueService,
    RunningVisualizer, STATUS_POLL_INTERVAL, ServerEvent, VisualizerLauncher,
};

/// Cada cuánto se redibuja el progreso y la cuenta regresiva de la reconexión.
const TICK_INTERVAL: Duration = Duration::from_secs(1);
const FIRST_ATTEMPT: u32 = 1;
/// Ticks entre relecturas del estado mientras suena. Se cuenta en ticks y no por
/// tiempo desde la última lectura, para que la demora de cada tick no la corra.
const STATUS_POLL_TICKS: usize =
    (STATUS_POLL_INTERVAL.as_secs() / TICK_INTERVAL.as_secs()) as usize;

/// Repositorios concretos que usa la app; los elige el composition root.
pub trait Backend {
    type Player: PlayerRepository + OptionsRepository + Send;
    type Queue: QueueRepository + Send;
    type Library: LibraryRepository + Send;
    type Playlists: PlaylistRepository + Send;
    /// Fuente de PCM del visualizador.
    type Pcm: PcmSource + Send + 'static;
}

pub struct Services<B: Backend> {
    pub playback: PlaybackService<B::Player>,
    pub queue: QueueService<B::Queue>,
    pub library: LibraryService<B::Library>,
    pub playlists: PlaylistService<B::Playlists>,
    pub visualizer: VisualizerLauncher<B::Pcm>,
    /// `None` con las letras desactivadas.
    pub lyrics: Option<LyricsLookup>,
    /// Buscar la letra de cada tema aunque la vista Letra no esté activa.
    pub lyrics_in_background: bool,
}

pub struct App<B: Backend> {
    services: Services<B>,
    server_events: UnboundedReceiver<ServerEvent>,
    appearance: Appearance,
    state: AppState,
    /// Solo mientras la vista Visual está activa.
    visualizer: Option<RunningVisualizer>,
}

impl<B: Backend> App<B> {
    pub fn new(
        services: Services<B>,
        server_events: UnboundedReceiver<ServerEvent>,
        appearance: Appearance,
        server: String,
    ) -> Self {
        let mut state = AppState::new(server);
        let format = services.visualizer.format();
        state.visual.source = services.visualizer.source_label().to_owned();
        state.visual.format = format.to_string();
        state.visual.channels = format.channels();
        state.visual.style = appearance.visualizer_style;
        state.lyrics_enabled = services.lyrics.is_some();
        Self {
            services,
            server_events,
            appearance,
            state,
            visualizer: None,
        }
    }

    /// Bucle principal: dibuja y espera el próximo evento (teclado, servidor o tick).
    pub async fn run<E>(
        mut self,
        terminal: &mut DefaultTerminal,
        mut terminal_events: E,
    ) -> io::Result<()>
    where
        E: Stream<Item = io::Result<Event>> + Unpin,
    {
        self.resynchronize().await;
        let mut tick = tokio::time::interval(TICK_INTERVAL);
        while self.state.running {
            // Antes de dibujar, para que un tema nuevo no muestre la letra del anterior.
            self.sync_lyrics();
            terminal.draw(|frame| {
                views::render(frame, &mut self.state, self.appearance, Instant::now());
            })?;
            self.sync_visualizer();
            let next_lyrics_line = self.next_lyrics_line();
            tokio::select! {
                event = terminal_events.next() => match event {
                    Some(event) => self.handle_terminal_event(event?).await,
                    None => self.state.running = false,
                },
                Some(event) = self.server_events.recv() => self.handle_server_event(event).await,
                Some(visual) = visual::next_state(&mut self.visualizer) => {
                    self.state.visual.state = visual;
                }
                Some(result) = lyrics::next_result(&mut self.services.lyrics) => {
                    self.apply_lyrics(result);
                }
                () = lyrics::sleep_until(next_lyrics_line) => {}
                _ = tick.tick() => self.handle_tick().await,
            }
        }
        Ok(())
    }

    async fn handle_terminal_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            self.handle_key(key).await;
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if self.state.prompt.is_some() {
            return self.handle_prompt_key(key).await;
        }
        if self.state.help_visible {
            self.state.help_visible = false;
            return;
        }
        if let Some(action) = action_for(key) {
            self.perform(action).await;
        }
    }

    async fn handle_server_event(&mut self, event: ServerEvent) {
        match event {
            ServerEvent::Changed(changed) => self.apply_changes(&changed).await,
            ServerEvent::Disconnected {
                attempt,
                retry_at,
                delay,
                reason,
            } => {
                self.state.connection = ConnectionState::Waiting {
                    attempt,
                    retry_at: retry_at.into_std(),
                    delay,
                };
                self.report_disconnection(attempt, reason);
            }
            ServerEvent::Connecting { attempt } => {
                self.state.connection = ConnectionState::Connecting { attempt };
            }
            ServerEvent::Connected => {
                self.state.connection = ConnectionState::Connected;
                self.resynchronize().await;
                self.notify_info(self.phrase(Phrase::ConnectionRecovered));
            }
        }
    }

    /// El primer corte se avisa en la línea inferior; los reintentos fallidos, solo en
    /// el log (el encabezado ya muestra la cuenta regresiva).
    fn report_disconnection(&mut self, attempt: u32, reason: Error) {
        let text = self.message(Message::ConnectionLost(&reason));
        if attempt == FIRST_ATTEMPT {
            self.notify_error(text);
        } else {
            tracing::warn!("{text}");
        }
    }

    async fn handle_tick(&mut self) {
        self.state.ticks = self.state.ticks.wrapping_add(1);
        let connected = self.state.connection == ConnectionState::Connected;
        if connected
            && self.state.player.needs_polling()
            && self.state.ticks.is_multiple_of(STATUS_POLL_TICKS)
        {
            self.refresh_status().await;
        }
    }

    async fn perform(&mut self, action: Action) {
        match action {
            Action::Quit => self.state.running = false,
            Action::ShowView(ViewKind::Lyrics) if !self.state.lyrics_enabled => {
                self.notify_info(self.phrase(Phrase::LyricsDisabled));
            }
            Action::ShowView(view) => self.show_view(view).await,
            Action::ShowVisualizer => self.show_visualizer().await,
            Action::NextView => self.show_view(self.state.next_view()).await,
            Action::PreviousView => self.show_view(self.state.previous_view()).await,
            Action::ToggleHelp => self.state.help_visible = !self.state.help_visible,
            Action::StartSearch => self.start_search().await,
            Action::ShowPlaying => self.show_playing().await,
            Action::RetryLyrics => self.retry_lyrics(),
            Action::SaveQueue => {
                self.state.prompt = Some(Prompt::text(PromptKind::SavePlaylist, String::new()));
            }
            Action::ClearQueue => {
                self.state.prompt = Some(Prompt::confirm(Confirmation::ClearQueue));
            }
            Action::Navigate(_)
            | Action::Activate
            | Action::Forward
            | Action::Back
            | Action::Add
            | Action::AddAll
            | Action::Delete => self.perform_in_view(action).await,
            _ => self.perform_playback(action).await,
        }
    }

    async fn perform_in_view(&mut self, action: Action) {
        match self.state.view {
            ViewKind::Queue => self.queue_action(action).await,
            ViewKind::Library => self.library_action(action).await,
            ViewKind::Browse => self.browse_action(action).await,
            ViewKind::Search => self.search_action(action).await,
            ViewKind::Playlists => self.playlists_action(action).await,
            ViewKind::Lyrics => self.lyrics_action(action).await,
            ViewKind::Visualizer => {}
        }
    }

    /// Devuelve el valor si salió bien; si no, muestra el error y lo registra.
    fn report<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.notify_error(self.message(Message::Failure(&error)));
                None
            }
        }
    }

    /// Un texto fijo en el idioma configurado, listo para un aviso.
    fn phrase(&self, phrase: Phrase) -> String {
        self.appearance.texts.phrase(phrase).to_owned()
    }

    fn message(&self, message: Message<'_>) -> String {
        self.appearance.texts.message(&message)
    }

    fn notify_info(&mut self, text: String) {
        self.notify(NoticeLevel::Info, text);
    }

    fn notify_error(&mut self, text: String) {
        tracing::warn!("{text}");
        self.notify(NoticeLevel::Error, text);
    }

    fn notify(&mut self, level: NoticeLevel, text: String) {
        self.state.notice = Some(Notice {
            level,
            text,
            shown_at: Instant::now(),
        });
    }
}
