//! Acciones de la vista de cola.

use super::{App, Backend};
use crate::domain::Song;
use crate::i18n::Message;
use crate::tui::action::Action;
use crate::tui::state::ViewKind;

impl<B: Backend> App<B> {
    pub(super) async fn queue_action(&mut self, action: Action) {
        match action {
            Action::Navigate(navigation) => {
                self.state.queue.navigate(navigation);
            }
            Action::Activate => self.play_selected().await,
            Action::Delete => self.delete_selected().await,
            _ => {}
        }
    }

    /// Muestra la cola con el tema que suena seleccionado.
    pub(super) async fn show_playing(&mut self) {
        self.show_view(ViewKind::Queue).await;
        let Some(current) = self.state.player.status.song_position else {
            return;
        };
        let queue = &mut self.state.queue;
        queue.select_where(|song| song.position == Some(current));
        queue.request_centering();
    }

    async fn play_selected(&mut self) {
        let Some(position) = self.selected_queue_position() else {
            return;
        };
        let result = self.services.playback.play_position(position).await;
        self.report(result);
    }

    async fn delete_selected(&mut self) {
        let Some(position) = self.selected_queue_position() else {
            return;
        };
        let result = self.services.queue.delete(position).await;
        self.report(result);
    }

    fn selected_queue_position(&self) -> Option<u32> {
        self.state.queue.selected()?.position
    }

    /// Agrega canciones y avisa cuántas; compartido por las vistas que listan temas.
    pub(super) async fn add_songs(&mut self, songs: Vec<Song>) {
        if songs.is_empty() {
            return;
        }
        let result = self.services.queue.add_songs(&songs).await;
        if self.report(result).is_some() {
            let added = match songs.as_slice() {
                [song] => Message::AddedToQueue(song.display_title()),
                many => Message::SongsAdded(many.len()),
            };
            self.notify_info(self.message(added));
        }
    }
}
