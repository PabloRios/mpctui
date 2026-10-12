//! Acciones de la biblioteca en columnas Artista → Álbum → Tema.

use super::{App, Backend};
use crate::i18n::Message;
use crate::tui::action::{Action, Navigation};
use crate::tui::list::SelectionPolicy;
use crate::tui::picker::PlaylistAddition;
use crate::tui::state::LibraryColumn;

/// Entre el artista y el álbum en el aviso de lo agregado.
const ARTIST_ALBUM_SEPARATOR: &str = " — ";

impl<B: Backend> App<B> {
    pub(super) async fn library_action(&mut self, action: Action) {
        let library = &mut self.state.library;
        match action {
            Action::Navigate(navigation) => self.library_navigate(navigation).await,
            Action::Forward => library.focus = library.focus.next(),
            Action::Back => library.focus = library.focus.previous(),
            Action::Activate if library.focus == LibraryColumn::Songs => {
                self.add_library_selection().await;
            }
            Action::Activate => library.focus = library.focus.next(),
            Action::Add => self.add_library_selection().await,
            Action::AddAll => self.add_selected_album().await,
            Action::AddToPlaylist => self.pick_playlist_for_library_selection().await,
            _ => {}
        }
    }

    /// Recarga las tres columnas conservando la selección.
    pub(super) async fn reload_library(&mut self) {
        let result = self.services.library.artists().await;
        if let Some(artists) = self.report(result) {
            self.state
                .library
                .artists
                .set_items(artists, SelectionPolicy::Keep);
            self.reload_albums(SelectionPolicy::Keep).await;
        }
    }

    /// Al cambiar de artista o álbum, las columnas de la derecha vuelven al principio.
    async fn library_navigate(&mut self, navigation: Navigation) {
        let library = &mut self.state.library;
        match library.focus {
            LibraryColumn::Artists => {
                if library.artists.navigate(navigation) {
                    self.reload_albums(SelectionPolicy::Reset).await;
                }
            }
            LibraryColumn::Albums => {
                if library.albums.navigate(navigation) {
                    self.reload_songs(SelectionPolicy::Reset).await;
                }
            }
            LibraryColumn::Songs => {
                library.songs.navigate(navigation);
            }
        }
    }

    async fn reload_albums(&mut self, policy: SelectionPolicy) {
        let Some(artist) = self.state.library.selected_artist() else {
            self.state.library.albums.clear();
            return self.reload_songs(policy).await;
        };
        let result = self.services.library.albums(&artist).await;
        if let Some(albums) = self.report(result) {
            self.state.library.albums.set_items(albums, policy);
        }
        self.reload_songs(policy).await;
    }

    async fn reload_songs(&mut self, policy: SelectionPolicy) {
        let Some((artist, album)) = self.state.library.selected_album() else {
            self.state.library.songs.clear();
            return;
        };
        let result = self.services.library.album_songs(&artist, &album).await;
        if let Some(songs) = self.report(result) {
            self.state.library.songs.set_items(songs, policy);
        }
    }

    /// Agrega lo seleccionado en la columna activa: artista, álbum o tema.
    async fn add_library_selection(&mut self) {
        match self.state.library.focus {
            LibraryColumn::Artists => self.add_selected_artist().await,
            LibraryColumn::Albums => self.add_selected_album().await,
            LibraryColumn::Songs => {
                let song = self.state.library.songs.selected().cloned();
                self.add_songs(song.into_iter().collect()).await;
            }
        }
    }

    /// Lo mismo que agregaría `a`: el artista, el álbum (sus temas ya están en la
    /// columna Tema) o el tema.
    async fn pick_playlist_for_library_selection(&mut self) {
        let library = &self.state.library;
        let addition = match library.focus {
            LibraryColumn::Artists => {
                let Some(artist) = library.selected_artist() else {
                    return;
                };
                let result = self.services.library.artist_songs(&artist).await;
                let Some(songs) = self.report(result) else {
                    return;
                };
                PlaylistAddition::songs(&songs, artist)
            }
            LibraryColumn::Albums => {
                let Some((artist, album)) = library.selected_album() else {
                    return;
                };
                let label = format!("{artist}{ARTIST_ALBUM_SEPARATOR}{album}");
                PlaylistAddition::songs(library.songs.items(), label)
            }
            LibraryColumn::Songs => {
                let Some(song) = library.songs.selected() else {
                    return;
                };
                PlaylistAddition::song(song)
            }
        };
        self.open_playlist_picker(addition).await;
    }

    async fn add_selected_artist(&mut self) {
        let Some(artist) = self.state.library.selected_artist() else {
            return;
        };
        let result = self.services.queue.add_artist(&artist).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::AddedToQueue(&artist)));
        }
    }

    async fn add_selected_album(&mut self) {
        let Some((artist, album)) = self.state.library.selected_album() else {
            return;
        };
        let result = self.services.queue.add_album(&artist, &album).await;
        if self.report(result).is_some() {
            let added = format!("{artist}{ARTIST_ALBUM_SEPARATOR}{album}");
            self.notify_info(self.message(Message::AddedToQueue(&added)));
        }
    }
}
