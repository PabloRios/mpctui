//! Sincronización del estado con el servidor.

use std::time::Instant;

use super::{App, Backend};
use crate::domain::{Status, Subsystem};
use crate::service::{PlaybackClock, UpdateTransition, update_transition};
use crate::tui::list::SelectionPolicy;
use crate::tui::state::ViewKind;
use crate::tui::views::format_duration;

/// Subsistemas que cambian lo que muestra la barra de estado.
const PLAYER_SUBSYSTEMS: [Subsystem; 4] = [
    Subsystem::Player,
    Subsystem::Mixer,
    Subsystem::Options,
    Subsystem::Queue,
];

impl<B: Backend> App<B> {
    /// Vuelve a leer todo lo visible: al arrancar y al recuperar la conexión.
    pub(super) async fn resynchronize(&mut self) {
        self.refresh_queue().await;
        self.refresh_player().await;
        self.load_view().await;
    }

    pub(super) async fn apply_changes(&mut self, changed: &[Subsystem]) {
        if changed.contains(&Subsystem::Queue) {
            self.refresh_queue().await;
        }
        if PLAYER_SUBSYSTEMS
            .iter()
            .any(|subsystem| changed.contains(subsystem))
        {
            self.refresh_player().await;
        } else if changed.contains(&Subsystem::Update) {
            // Solo el estado dice si la actualización empezó o terminó.
            self.refresh_status().await;
        }
        if changed.contains(&Subsystem::Database)
            && matches!(self.state.view, ViewKind::Library | ViewKind::Browse)
        {
            self.load_view().await;
        }
        if changed.contains(&Subsystem::StoredPlaylist) && self.state.view == ViewKind::Playlists {
            self.load_view().await;
        }
    }

    pub(super) async fn show_view(&mut self, view: ViewKind) {
        self.state.view = view;
        self.load_view().await;
    }

    /// Recarga los datos de la vista activa conservando la selección.
    pub(super) async fn load_view(&mut self) {
        match self.state.view {
            ViewKind::Queue | ViewKind::Search | ViewKind::Visualizer => {}
            ViewKind::Library => self.reload_library().await,
            ViewKind::Browse => {
                let path = self.state.browse.path.clone();
                self.open_directory(path, SelectionPolicy::Keep).await;
            }
            ViewKind::Playlists => self.reload_playlists().await,
        }
    }

    async fn refresh_player(&mut self) {
        let result = self.services.playback.snapshot().await;
        if let Some(snapshot) = self.report(result) {
            self.state.player.current_song = snapshot.current_song;
            self.apply_status(snapshot.status);
        }
    }

    /// Relectura periódica mientras suena: bitrate y deriva del reloj.
    pub(super) async fn refresh_status(&mut self) {
        let result = self.services.playback.status().await;
        if let Some(status) = self.report(result) {
            self.apply_status(status);
        }
    }

    /// El reloj local se resincroniza con cada estado leído, y se ve si una
    /// actualización de la base empezó o terminó.
    fn apply_status(&mut self, status: Status) {
        let now = Instant::now();
        let transition = update_transition(&self.state.player.status, &status);
        self.state.clock = PlaybackClock::new(&status, now);
        self.state.player.status = status;
        match transition {
            Some(UpdateTransition::Started) => self.state.database_update = Some(now),
            Some(UpdateTransition::Finished) => self.finish_database_update(now),
            None => {}
        }
    }

    /// `Biblioteca actualizada (1:05)`; el tiempo cuenta desde que mpctui la vio.
    fn finish_database_update(&mut self, now: Instant) {
        let Some(started) = self.state.database_update.take() else {
            return;
        };
        let elapsed = format_duration(now.saturating_duration_since(started));
        self.notify_info(format!("Biblioteca actualizada ({elapsed})"));
    }

    async fn refresh_queue(&mut self) {
        let result = self.services.queue.songs().await;
        if let Some(songs) = self.report(result) {
            self.state.queue.set_items(songs, SelectionPolicy::Keep);
        }
    }
}
