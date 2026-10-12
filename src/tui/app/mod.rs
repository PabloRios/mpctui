//! Controlador de la TUI: recibe eventos, llama a los servicios y actualiza el estado.
//! Cada vista tiene su propio submódulo con sus acciones.

mod browse;
mod library;
mod playback;
mod playlists;
mod prompt;
mod queue;
mod refresh;
mod search;
mod spectrum;

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
use crate::repository::{
    LibraryRepository, OptionsRepository, PcmSource, PlayerRepository, PlaylistRepository,
    QueueRepository, Result,
};
use crate::service::{
    LibraryService, PlaybackService, PlaylistService, QueueService, RunningVisualizer,
    STATUS_POLL_INTERVAL, ServerEvent, VisualizerLauncher,
};

/// Cada cuánto se redibuja el progreso y la cuenta regresiva de la reconexión.
const TICK_INTERVAL: Duration = Duration::from_secs(1);
const CONNECTION_RECOVERED: &str = "Conexión recuperada";
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
}

pub struct App<B: Backend> {
    services: Services<B>,
    server_events: UnboundedReceiver<ServerEvent>,
    appearance: Appearance,
    state: AppState,
    /// Solo mientras la vista Espectro está activa.
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
        state.spectrum.source = services.visualizer.source_label().to_owned();
        state.spectrum.format = services.visualizer.format().to_string();
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
            let area = terminal
                .draw(|frame| {
                    views::render(frame, &mut self.state, self.appearance, Instant::now());
                })?
                .area;
            self.sync_visualizer(area);
            tokio::select! {
                event = terminal_events.next() => match event {
                    Some(event) => self.handle_terminal_event(event?).await,
                    None => self.state.running = false,
                },
                Some(event) = self.server_events.recv() => self.handle_server_event(event).await,
                Some(spectrum) = spectrum::next_state(&mut self.visualizer) => {
                    self.state.spectrum.state = spectrum;
                }
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
                self.notify_info(CONNECTION_RECOVERED.into());
            }
        }
    }

    /// El primer corte se avisa en la línea inferior; los reintentos fallidos, solo en
    /// el log (el encabezado ya muestra la cuenta regresiva).
    fn report_disconnection(&mut self, attempt: u32, reason: String) {
        let text = format!("Sin conexión con MPD: {reason}");
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
            Action::ShowView(view) => self.show_view(view).await,
            Action::NextView => self.show_view(self.state.view.next()).await,
            Action::PreviousView => self.show_view(self.state.view.previous()).await,
            Action::ToggleHelp => self.state.help_visible = !self.state.help_visible,
            Action::StartSearch => self.start_search().await,
            Action::ShowPlaying => self.show_playing().await,
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
            ViewKind::Spectrum => {}
        }
    }

    /// Devuelve el valor si salió bien; si no, muestra el error y lo registra.
    fn report<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.notify_error(error.to_string());
                None
            }
        }
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
