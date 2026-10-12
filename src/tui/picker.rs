//! Selector de playlists: a cuál agregar lo seleccionado, o una nueva.

use crossterm::event::{KeyCode, KeyEvent};

use super::action::{Action, action_for};
use super::list::{SelectableList, SelectionPolicy};
use crate::domain::{PlaylistName, Song};

/// Lo que se agrega a una playlist y cómo nombrarlo en el aviso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistAddition {
    /// Archivos o directorios, en el orden en que se agregan.
    pub uris: Vec<String>,
    pub label: String,
}

impl PlaylistAddition {
    /// Un tema: su archivo, nombrado por su título.
    pub fn song(song: &Song) -> Self {
        Self::songs(std::slice::from_ref(song), song.display_title().to_owned())
    }

    /// Varios temas nombrados como un todo (artista, álbum, playlist).
    pub fn songs(songs: &[Song], label: String) -> Self {
        Self {
            uris: songs.iter().map(|song| song.file.clone()).collect(),
            label,
        }
    }

    /// Un archivo o un directorio completo, nombrado por su ruta.
    pub fn path(path: String) -> Self {
        Self {
            uris: vec![path.clone()],
            label: path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaylistChoice {
    /// Pedir el nombre de una playlist nueva.
    New,
    Existing(PlaylistName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOutcome {
    Browsing,
    Chosen(PlaylistChoice),
    Cancelled,
}

#[derive(Debug)]
pub struct PlaylistPicker {
    pub addition: PlaylistAddition,
    pub choices: SelectableList<PlaylistChoice>,
}

impl PlaylistPicker {
    /// «Nueva playlist» primero (así siempre hay algo que elegir) y después las
    /// existentes, en el orden recibido.
    pub fn new(addition: PlaylistAddition, names: Vec<PlaylistName>) -> Self {
        let mut choices = SelectableList::default();
        let items = std::iter::once(PlaylistChoice::New)
            .chain(names.into_iter().map(PlaylistChoice::Existing))
            .collect();
        choices.set_items(items, SelectionPolicy::Reset);
        Self { addition, choices }
    }

    /// Las teclas de navegación son las de la tabla de atajos.
    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome {
        match key.code {
            KeyCode::Enter => self
                .choices
                .selected()
                .cloned()
                .map_or(PickerOutcome::Browsing, PickerOutcome::Chosen),
            KeyCode::Esc => PickerOutcome::Cancelled,
            _ => {
                if let Some(Action::Navigate(navigation)) = action_for(key) {
                    self.choices.navigate(navigation);
                }
                PickerOutcome::Browsing
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;

    fn picker(names: &[&str]) -> PlaylistPicker {
        let addition = PlaylistAddition {
            uris: vec!["a.flac".into()],
            label: "A".into(),
        };
        let names = names
            .iter()
            .map(|name| PlaylistName::new(name).unwrap())
            .collect();
        PlaylistPicker::new(addition, names)
    }

    fn press(picker: &mut PlaylistPicker, code: KeyCode) -> PickerOutcome {
        picker.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn new_playlist_comes_first_and_is_chosen_by_default() {
        let mut picker = picker(&[]);
        assert_eq!(
            press(&mut picker, KeyCode::Enter),
            PickerOutcome::Chosen(PlaylistChoice::New)
        );
    }

    #[test]
    fn navigates_with_the_list_keys_and_chooses_with_enter() {
        let mut picker = picker(&["Rock", "Jazz"]);
        assert_eq!(
            press(&mut picker, KeyCode::Char('j')),
            PickerOutcome::Browsing
        );
        assert_eq!(press(&mut picker, KeyCode::Down), PickerOutcome::Browsing);
        assert_eq!(
            press(&mut picker, KeyCode::Enter),
            PickerOutcome::Chosen(PlaylistChoice::Existing(PlaylistName::new("Jazz").unwrap()))
        );
        press(&mut picker, KeyCode::Char('g'));
        assert_eq!(picker.choices.selected(), Some(&PlaylistChoice::New));
    }

    #[test]
    fn escape_cancels_and_other_keys_do_nothing() {
        let mut picker = picker(&["Rock"]);
        assert_eq!(
            press(&mut picker, KeyCode::Char('d')),
            PickerOutcome::Browsing
        );
        assert_eq!(picker.choices.selected(), Some(&PlaylistChoice::New));
        assert_eq!(press(&mut picker, KeyCode::Esc), PickerOutcome::Cancelled);
    }
}
