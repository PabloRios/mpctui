//! Selectores: el de playlists se abre con lo que hay que agregar y, al elegir, lo
//! agrega; el de particiones mueve la salida elegida.

use crossterm::event::KeyEvent;

use super::{App, Backend};
use crate::domain::{PlaylistName, Song};
use crate::i18n::Message;
use crate::tui::picker::{Picker, PickerOutcome, PlaylistAddition, PlaylistChoice, PlaylistPicker};
use crate::tui::prompt::{Prompt, PromptKind};

impl<B: Backend> App<B> {
    /// Abre el selector con las playlists del servidor; sin nada que agregar, no abre.
    pub(super) async fn open_playlist_picker(&mut self, addition: PlaylistAddition) {
        if addition.uris.is_empty() {
            return;
        }
        let result = self.services.playlists.names().await;
        if let Some(names) = self.report(result) {
            self.state.picker = Some(Picker::Playlist(PlaylistPicker::new(addition, names)));
        }
    }

    /// Compartido por las vistas que listan temas.
    pub(super) async fn pick_playlist_for_song(&mut self, song: Option<Song>) {
        if let Some(song) = song {
            self.open_playlist_picker(PlaylistAddition::song(&song))
                .await;
        }
    }

    pub(super) async fn handle_picker_key(&mut self, key: KeyEvent) {
        match self.state.picker.as_mut() {
            Some(Picker::Playlist(picker)) => match picker.handle_key(key) {
                PickerOutcome::Browsing => {}
                PickerOutcome::Cancelled => self.state.picker = None,
                PickerOutcome::Chosen(choice) => {
                    if let Some(Picker::Playlist(picker)) = self.state.picker.take() {
                        self.apply_choice(choice, picker.addition).await;
                    }
                }
            },
            Some(Picker::Partition(picker)) => match picker.handle_key(key) {
                PickerOutcome::Browsing => {}
                PickerOutcome::Cancelled => self.state.picker = None,
                PickerOutcome::Chosen(destination) => {
                    if let Some(Picker::Partition(picker)) = self.state.picker.take() {
                        self.move_output(&picker.output, &destination).await;
                    }
                }
            },
            None => {}
        }
    }

    /// Recibe el nombre ya validado (elegido o escrito en el diálogo).
    pub(super) async fn add_to_playlist(
        &mut self,
        name: &PlaylistName,
        addition: &PlaylistAddition,
    ) {
        let result = self.services.playlists.add_uris(name, &addition.uris).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::AddedToPlaylist {
                what: &addition.label,
                playlist: name.as_str(),
            }));
        }
    }

    /// Una playlist nueva pide el nombre antes de agregar.
    async fn apply_choice(&mut self, choice: PlaylistChoice, addition: PlaylistAddition) {
        match choice {
            PlaylistChoice::Existing(name) => self.add_to_playlist(&name, &addition).await,
            PlaylistChoice::New => {
                let kind = PromptKind::NewPlaylist(addition);
                self.state.prompt = Some(Prompt::text(kind, String::new()));
            }
        }
    }
}
