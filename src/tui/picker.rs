//! Selectores: a qué playlist agregar lo seleccionado (o a una nueva), a qué partición
//! mover una salida y por qué criterios ordenar la cola. Todos se navegan igual.

use crossterm::event::{KeyCode, KeyEvent};

use super::action::{Action, action_for};
use super::list::{SelectableList, SelectionPolicy};
use crate::domain::{Output, PartitionName, PlaylistName, Song, SortKey};

/// Agrega o quita el criterio resaltado en el selector de orden.
const TOGGLE_KEY: KeyCode = KeyCode::Char(' ');

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
pub enum PickerOutcome<T> {
    Browsing,
    Chosen(T),
    Cancelled,
}

/// El selector abierto.
#[derive(Debug)]
pub enum Picker {
    Playlist(PlaylistPicker),
    Partition(PartitionPicker),
    Sort(SortPicker),
}

/// Enter elige, Esc cancela y las teclas de navegación son las de la tabla de atajos.
fn choose<T: Clone>(choices: &mut SelectableList<T>, key: KeyEvent) -> PickerOutcome<T> {
    match key.code {
        KeyCode::Enter => choices
            .selected()
            .cloned()
            .map_or(PickerOutcome::Browsing, PickerOutcome::Chosen),
        KeyCode::Esc => PickerOutcome::Cancelled,
        _ => {
            if let Some(Action::Navigate(navigation)) = action_for(key) {
                choices.navigate(navigation);
            }
            PickerOutcome::Browsing
        }
    }
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

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome<PlaylistChoice> {
        choose(&mut self.choices, key)
    }
}

/// A qué partición mover una salida: las demás, en el orden recibido.
#[derive(Debug)]
pub struct PartitionPicker {
    pub output: Output,
    pub choices: SelectableList<PartitionName>,
}

impl PartitionPicker {
    pub fn new(output: Output, destinations: Vec<PartitionName>) -> Self {
        let mut choices = SelectableList::default();
        choices.set_items(destinations, SelectionPolicy::Reset);
        Self { output, choices }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome<PartitionName> {
        choose(&mut self.choices, key)
    }
}

/// Los criterios de orden de la cola, elegidos de a uno: el orden en que se eligen es el
/// orden en que se aplican.
#[derive(Debug)]
pub struct SortPicker {
    pub choices: SelectableList<SortKey>,
    pub order: SortOrder,
}

impl Default for SortPicker {
    fn default() -> Self {
        let mut choices = SelectableList::default();
        choices.set_items(SortKey::ALL.to_vec(), SelectionPolicy::Reset);
        Self {
            choices,
            order: SortOrder::default(),
        }
    }
}

impl SortPicker {
    /// Espacio elige o descarta; Enter ordena por los elegidos o, si no hay ninguno, por
    /// el resaltado.
    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome<Vec<SortKey>> {
        match key.code {
            TOGGLE_KEY => {
                if let Some(&key) = self.choices.selected() {
                    self.order.toggle(key);
                }
                PickerOutcome::Browsing
            }
            KeyCode::Enter => self
                .criteria()
                .map_or(PickerOutcome::Browsing, PickerOutcome::Chosen),
            _ => match choose(&mut self.choices, key) {
                PickerOutcome::Cancelled => PickerOutcome::Cancelled,
                PickerOutcome::Browsing | PickerOutcome::Chosen(_) => PickerOutcome::Browsing,
            },
        }
    }

    fn criteria(&self) -> Option<Vec<SortKey>> {
        if self.order.keys.is_empty() {
            return self.choices.selected().map(|key| vec![*key]);
        }
        Some(self.order.keys.clone())
    }
}

/// Los criterios elegidos, sin repetir, en el orden en que se eligieron.
#[derive(Debug, Default)]
pub struct SortOrder {
    keys: Vec<SortKey>,
}

impl SortOrder {
    /// Lo agrega al final o, si ya estaba, lo quita (los siguientes suben un lugar).
    fn toggle(&mut self, key: SortKey) {
        match self.keys.iter().position(|chosen| *chosen == key) {
            Some(index) => {
                self.keys.remove(index);
            }
            None => self.keys.push(key),
        }
    }

    /// El lugar del criterio en el orden, desde 1, si está elegido.
    pub fn rank(&self, key: SortKey) -> Option<usize> {
        let index = self.keys.iter().position(|chosen| *chosen == key)?;
        Some(index + 1)
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

    fn press(picker: &mut PlaylistPicker, code: KeyCode) -> PickerOutcome<PlaylistChoice> {
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

    #[test]
    fn the_partition_picker_chooses_a_destination() {
        let output = Output {
            id: 0,
            name: "pulse".into(),
            plugin: "pulse".into(),
            enabled: true,
        };
        let destinations = ["cocina", "living"]
            .iter()
            .map(|name| PartitionName::new(name).unwrap())
            .collect();
        let mut picker = PartitionPicker::new(output, destinations);
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        assert_eq!(
            picker.handle_key(key(KeyCode::Char('j'))),
            PickerOutcome::Browsing
        );
        assert_eq!(
            picker.handle_key(key(KeyCode::Enter)),
            PickerOutcome::Chosen(PartitionName::new("living").unwrap())
        );
        assert_eq!(
            picker.handle_key(key(KeyCode::Esc)),
            PickerOutcome::Cancelled
        );
    }

    fn press_sort(picker: &mut SortPicker, code: KeyCode) -> PickerOutcome<Vec<SortKey>> {
        picker.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn space_picks_criteria_in_order_and_enter_sorts_by_them() {
        let mut picker = SortPicker::default();
        press_sort(&mut picker, KeyCode::Char('j'));
        press_sort(&mut picker, TOGGLE_KEY);
        press_sort(&mut picker, KeyCode::Char('k'));
        press_sort(&mut picker, TOGGLE_KEY);
        assert_eq!(picker.order.rank(SortKey::Year), Some(1));
        assert_eq!(picker.order.rank(SortKey::Artist), Some(2));
        assert_eq!(picker.order.rank(SortKey::Album), None);
        assert_eq!(
            press_sort(&mut picker, KeyCode::Enter),
            PickerOutcome::Chosen(vec![SortKey::Year, SortKey::Artist])
        );
    }

    #[test]
    fn space_again_drops_the_criterion_and_renumbers_the_rest() {
        let mut picker = SortPicker::default();
        press_sort(&mut picker, TOGGLE_KEY);
        press_sort(&mut picker, KeyCode::Char('j'));
        press_sort(&mut picker, TOGGLE_KEY);
        press_sort(&mut picker, KeyCode::Char('k'));
        press_sort(&mut picker, TOGGLE_KEY);
        assert_eq!(picker.order.rank(SortKey::Artist), None);
        assert_eq!(picker.order.rank(SortKey::Year), Some(1));
    }

    #[test]
    fn enter_without_criteria_sorts_by_the_highlighted_one() {
        let mut picker = SortPicker::default();
        press_sort(&mut picker, KeyCode::Char('j'));
        press_sort(&mut picker, KeyCode::Char('j'));
        assert_eq!(
            press_sort(&mut picker, KeyCode::Enter),
            PickerOutcome::Chosen(vec![SortKey::Album])
        );
    }

    #[test]
    fn escape_cancels_the_sort() {
        let mut picker = SortPicker::default();
        press_sort(&mut picker, TOGGLE_KEY);
        assert_eq!(
            press_sort(&mut picker, KeyCode::Esc),
            PickerOutcome::Cancelled
        );
    }
}
