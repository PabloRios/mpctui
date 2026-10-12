//! Acciones de la vista de playlists guardadas: con foco en los nombres se cargan y se
//! borran playlists enteras; con foco en los temas se edita la seleccionada.

use super::{App, Backend};
use crate::domain::{MoveDirection, PlaylistName};
use crate::i18n::Message;
use crate::tui::action::{Action, Navigation};
use crate::tui::list::SelectionPolicy;
use crate::tui::picker::PlaylistAddition;
use crate::tui::prompt::{Confirmation, Prompt};
use crate::tui::state::PlaylistsColumn;

impl<B: Backend> App<B> {
    pub(super) async fn playlists_action(&mut self, action: Action) {
        match self.state.playlists.focus {
            PlaylistsColumn::Names => self.playlist_names_action(action).await,
            PlaylistsColumn::Songs => self.playlist_songs_action(action).await,
        }
    }

    /// Recarga conservando la playlist elegida aunque cambie de lugar (otra nueva puede
    /// ordenarse antes); si ya no está, el contenido vuelve al principio.
    pub(super) async fn reload_playlists(&mut self) {
        let result = self.services.playlists.names().await;
        let Some(names) = self.report(result) else {
            return;
        };
        let previous = self.selected_playlist();
        let list = &mut self.state.playlists.names;
        list.set_items(names, SelectionPolicy::Keep);
        if let Some(previous) = &previous {
            list.select_where(|name| name == previous);
        }
        let policy = if self.selected_playlist() == previous {
            SelectionPolicy::Keep
        } else {
            SelectionPolicy::Reset
        };
        self.reload_preview(policy).await;
    }

    /// Recibe el nombre ya validado por el diálogo.
    pub(super) async fn save_queue(&mut self, name: &PlaylistName) {
        let result = self.services.playlists.save_queue(name).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::QueueSaved(name.as_str())));
        }
    }

    pub(super) async fn remove_playlist(&mut self, name: &PlaylistName) {
        let result = self.services.playlists.remove(name).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::PlaylistDeleted(name.as_str())));
        }
    }

    async fn playlist_names_action(&mut self, action: Action) {
        match action {
            Action::Navigate(navigation) => {
                if self.state.playlists.names.navigate(navigation) {
                    self.reload_preview(SelectionPolicy::Reset).await;
                }
            }
            Action::Forward if self.selected_playlist().is_some() => {
                self.state.playlists.focus = PlaylistsColumn::Songs;
            }
            Action::Activate | Action::Add => self.load_selected_playlist().await,
            Action::Delete => {
                if let Some(name) = self.selected_playlist() {
                    let confirmation = Confirmation::RemovePlaylist(name);
                    self.state.prompt = Some(Prompt::confirm(confirmation));
                }
            }
            Action::AddToPlaylist => {
                if let Some(name) = self.selected_playlist() {
                    let songs = self.state.playlists.preview.items();
                    let addition = PlaylistAddition::songs(songs, name.as_str().to_owned());
                    self.open_playlist_picker(addition).await;
                }
            }
            _ => {}
        }
    }

    async fn playlist_songs_action(&mut self, action: Action) {
        let preview = &mut self.state.playlists.preview;
        match action {
            Action::Navigate(navigation) => {
                preview.navigate(navigation);
            }
            Action::Back => self.state.playlists.focus = PlaylistsColumn::Names,
            Action::Activate | Action::Add => {
                let song = preview.selected().cloned();
                self.add_songs(song.into_iter().collect()).await;
            }
            Action::Delete => self.delete_selected_playlist_song().await,
            Action::MoveSong(direction) => self.move_selected_playlist_song(direction).await,
            Action::AddToPlaylist => {
                let song = preview.selected().cloned();
                self.pick_playlist_for_song(song).await;
            }
            _ => {}
        }
    }

    async fn load_selected_playlist(&mut self) {
        let Some(name) = self.selected_playlist() else {
            return;
        };
        let result = self.services.playlists.load(&name).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::PlaylistLoaded(name.as_str())));
        }
    }

    async fn delete_selected_playlist_song(&mut self) {
        let Some((name, position)) = self.selected_playlist_song() else {
            return;
        };
        let title = self
            .state
            .playlists
            .preview
            .selected()
            .map(|song| song.display_title().to_owned())
            .unwrap_or_default();
        let result = self.services.playlists.delete_song(&name, position).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::RemovedFromPlaylist {
                song: &title,
                playlist: name.as_str(),
            }));
        }
    }

    /// Como en la cola: posiciones, y el cursor acompaña al tema.
    async fn move_selected_playlist_song(&mut self, direction: MoveDirection) {
        let Some((name, position)) = self.selected_playlist_song() else {
            return;
        };
        let length = self.state.playlists.preview.items().len();
        let result = self
            .services
            .playlists
            .shift(&name, position, direction, length)
            .await;
        if let Some(Some(_)) = self.report(result) {
            let following = Navigation::following(direction);
            self.state.playlists.preview.navigate(following);
        }
    }

    async fn reload_preview(&mut self, policy: SelectionPolicy) {
        let Some(name) = self.selected_playlist() else {
            self.state.playlists.preview.clear();
            self.state.playlists.focus = PlaylistsColumn::Names;
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

    /// La playlist elegida y la posición del tema seleccionado en ella.
    fn selected_playlist_song(&self) -> Option<(PlaylistName, u32)> {
        let index = self.state.playlists.preview.selected_index()?;
        Some((self.selected_playlist()?, u32::try_from(index).ok()?))
    }
}
