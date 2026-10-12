//! Acciones de la vista de playlists guardadas.

use super::{App, Backend};
use crate::domain::PlaylistName;
use crate::tui::action::Action;
use crate::tui::list::SelectionPolicy;
use crate::tui::prompt::{Confirmation, Prompt};

impl<B: Backend> App<B> {
    pub(super) async fn playlists_action(&mut self, action: Action) {
        match action {
            Action::Navigate(navigation) => {
                if self.state.playlists.names.navigate(navigation) {
                    self.reload_preview(SelectionPolicy::Reset).await;
                }
            }
            Action::Activate | Action::Add => self.load_selected_playlist().await,
            Action::Delete => {
                if let Some(name) = self.selected_playlist() {
                    let confirmation = Confirmation::RemovePlaylist(name);
                    self.state.prompt = Some(Prompt::confirm(confirmation));
                }
            }
            _ => {}
        }
    }

    pub(super) async fn reload_playlists(&mut self) {
        let result = self.services.playlists.names().await;
        if let Some(names) = self.report(result) {
            self.state
                .playlists
                .names
                .set_items(names, SelectionPolicy::Keep);
            self.reload_preview(SelectionPolicy::Keep).await;
        }
    }

    /// Recibe el nombre ya validado por el diálogo.
    pub(super) async fn save_queue(&mut self, name: &PlaylistName) {
        let result = self.services.playlists.save_queue(name).await;
        if self.report(result).is_some() {
            self.notify_info(format!("Cola guardada como «{name}»"));
        }
    }

    pub(super) async fn remove_playlist(&mut self, name: &PlaylistName) {
        let result = self.services.playlists.remove(name).await;
        if self.report(result).is_some() {
            self.notify_info(format!("Playlist «{name}» borrada"));
        }
    }

    async fn load_selected_playlist(&mut self) {
        let Some(name) = self.selected_playlist() else {
            return;
        };
        let result = self.services.playlists.load(&name).await;
        if self.report(result).is_some() {
            self.notify_info(format!("Playlist «{name}» agregada a la cola"));
        }
    }

    async fn reload_preview(&mut self, policy: SelectionPolicy) {
        let Some(name) = self.selected_playlist() else {
            self.state.playlists.preview.clear();
            return;
        };
        let result = self.services.playlists.songs(&name).await;
        if let Some(songs) = self.report(result) {
            self.state.playlists.preview.set_items(songs, policy);
        }
    }

    fn selected_playlist(&self) -> Option<PlaylistName> {
        self.state.playlists.names.selected().cloned()
    }
}
