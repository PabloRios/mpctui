//! Ficha del tema (`i`): todos sus tags y los datos del archivo. Se abre sobre el tema
//! elegido o, en las vistas sin un tema elegido, sobre el que suena.

use crossterm::event::KeyEvent;

use super::{App, Backend, PAGE_LINES};
use crate::domain::{Entry, Song};
use crate::tui::action::{Action, action_for};
use crate::tui::state::{LibraryColumn, PlaylistsColumn, SongInfoView, ViewKind};

impl<B: Backend> App<B> {
    /// Sin tema elegido ni sonando, no abre nada.
    pub(super) async fn show_song_info(&mut self) {
        let Some(song) = self.song_for_info() else {
            return;
        };
        let result = self.services.library.song_details(&song).await;
        if let Some(details) = self.report(result) {
            self.state.song_info = Some(SongInfoView::new(song, details));
        }
    }

    /// Las teclas de navegación desplazan la ficha; cualquier otra la cierra.
    pub(super) fn handle_song_info_key(&mut self, key: KeyEvent) {
        let Some(info) = self.state.song_info.as_mut() else {
            return;
        };
        match action_for(key) {
            Some(Action::Navigate(navigation)) => {
                info.scroll = navigation.apply(info.scroll, PAGE_LINES);
            }
            _ => self.state.song_info = None,
        }
    }

    fn song_for_info(&self) -> Option<Song> {
        self.selected_song()
            .or(self.state.player.current_song.as_ref())
            .cloned()
    }

    /// El tema elegido en la vista activa, si lo elegido es un tema.
    fn selected_song(&self) -> Option<&Song> {
        let state = &self.state;
        match state.view {
            ViewKind::Queue => state.queue.selected(),
            ViewKind::Library if state.library.focus == LibraryColumn::Songs => {
                state.library.songs.selected()
            }
            ViewKind::Browse => match state.browse.entries.selected() {
                Some(Entry::File(song)) => Some(song),
                _ => None,
            },
            ViewKind::Search => state.search.results.selected(),
            ViewKind::Playlists if state.playlists.focus == PlaylistsColumn::Songs => {
                state.playlists.preview.selected()
            }
            _ => None,
        }
    }
}
