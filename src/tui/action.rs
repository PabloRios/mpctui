//! Mapa de atajos: una sola tabla de la que salen la traducción de teclas y la ayuda.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::glyphs::{ArrowKeys, Glyphs};
use super::state::ViewKind;
use crate::domain::MoveDirection;
use crate::i18n::{Catalog, Phrase};

/// Nombres de teclas que se escriben igual en todos los idiomas.
const CONTROL_PREFIX: &str = "Ctrl+";
const ENTER_KEY: &str = "Enter";
const TAB_KEY: &str = "Tab";
const BACK_TAB_KEY: &str = "Shift+Tab";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    Next,
    Previous,
    PageDown,
    PageUp,
    First,
    Last,
}

impl Navigation {
    /// El paso del cursor que acompaña a un tema que se corre un lugar.
    pub fn following(direction: MoveDirection) -> Self {
        match direction {
            MoveDirection::Up => Navigation::Previous,
            MoveDirection::Down => Navigation::Next,
        }
    }

    /// El índice al que lleva desde `index`, con páginas de `page` filas. `Last` da
    /// `usize::MAX`: quien llama lo limita al largo de lo que recorre.
    pub fn apply(self, index: usize, page: usize) -> usize {
        match self {
            Navigation::Next => index.saturating_add(1),
            Navigation::Previous => index.saturating_sub(1),
            Navigation::PageDown => index.saturating_add(page),
            Navigation::PageUp => index.saturating_sub(page),
            Navigation::First => 0,
            Navigation::Last => usize::MAX,
        }
    }
}

/// Acciones del usuario. Las de vista (`Activate`, `Add`, `Delete`…) las interpreta
/// la vista activa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ShowView(ViewKind),
    /// Entra al visualizador o, si ya está, pasa a la forma siguiente.
    ShowVisualizer,
    NextView,
    PreviousView,
    ToggleHelp,
    Navigate(Navigation),
    Activate,
    Forward,
    Back,
    Add,
    AddAll,
    Delete,
    /// Corre el tema seleccionado un lugar (cola o playlist).
    MoveSong(MoveDirection),
    /// Abre el selector de playlists para agregar lo seleccionado.
    AddToPlaylist,
    /// En la cola, abre el diálogo para elegir los criterios de orden.
    SortQueue,
    TogglePause,
    Stop,
    NextSong,
    PreviousSong,
    VolumeUp,
    VolumeDown,
    SeekForward,
    SeekBackward,
    ClearQueue,
    SaveQueue,
    StartSearch,
    ToggleRandom,
    ToggleRepeat,
    CycleSingle,
    ToggleConsume,
    ShowPlaying,
    /// En la vista Letra, vuelve a buscar la letra si la búsqueda falló.
    RetryLyrics,
    UpdateDatabase,
    /// En la vista Salidas, pide el nombre de una partición nueva.
    NewPartition,
    /// En la vista Salidas, abre el selector de partición para la salida elegida.
    MoveOutput,
    /// Pasa al tema de colores siguiente (no se guarda al salir).
    NextTheme,
    /// Abre la ficha del tema elegido o, si no hay, del que suena.
    ShowSongInfo,
}

/// Las vistas de listas comparten descripción (`Phrase::GoToView`): la ayuda las
/// muestra en una sola línea.
pub struct Binding {
    pub keys: &'static [KeyCode],
    pub control: bool,
    pub action: Action,
    pub description: Phrase,
}

const fn plain(keys: &'static [KeyCode], action: Action, description: Phrase) -> Binding {
    Binding {
        keys,
        control: false,
        action,
        description,
    }
}

const fn control(keys: &'static [KeyCode], action: Action, description: Phrase) -> Binding {
    Binding {
        keys,
        control: true,
        action,
        description,
    }
}

pub const BINDINGS: &[Binding] = &[
    plain(&[KeyCode::Char('q')], Action::Quit, Phrase::Quit),
    control(&[KeyCode::Char('c')], Action::Quit, Phrase::Quit),
    plain(
        &[KeyCode::Char('?')],
        Action::ToggleHelp,
        Phrase::ToggleHelp,
    ),
    plain(
        &[KeyCode::Char('1')],
        Action::ShowView(ViewKind::Queue),
        Phrase::GoToView,
    ),
    plain(
        &[KeyCode::Char('2')],
        Action::ShowView(ViewKind::Library),
        Phrase::GoToView,
    ),
    plain(
        &[KeyCode::Char('3')],
        Action::ShowView(ViewKind::Browse),
        Phrase::GoToView,
    ),
    plain(
        &[KeyCode::Char('4')],
        Action::ShowView(ViewKind::Search),
        Phrase::GoToView,
    ),
    plain(
        &[KeyCode::Char('5')],
        Action::ShowView(ViewKind::Playlists),
        Phrase::GoToView,
    ),
    plain(
        &[KeyCode::Char('6')],
        Action::ShowVisualizer,
        Phrase::CycleVisualizer,
    ),
    plain(
        &[KeyCode::Char('7')],
        Action::ShowView(ViewKind::Lyrics),
        Phrase::GoToView,
    ),
    plain(
        &[KeyCode::Char('8')],
        Action::ShowView(ViewKind::Outputs),
        Phrase::GoToView,
    ),
    plain(&[KeyCode::Tab], Action::NextView, Phrase::CycleViews),
    plain(
        &[KeyCode::BackTab],
        Action::PreviousView,
        Phrase::CycleViews,
    ),
    plain(
        &[KeyCode::Char('j'), KeyCode::Down],
        Action::Navigate(Navigation::Next),
        Phrase::MoveDown,
    ),
    plain(
        &[KeyCode::Char('k'), KeyCode::Up],
        Action::Navigate(Navigation::Previous),
        Phrase::MoveUp,
    ),
    plain(
        &[KeyCode::PageDown],
        Action::Navigate(Navigation::PageDown),
        Phrase::PageDown,
    ),
    control(
        &[KeyCode::Char('d')],
        Action::Navigate(Navigation::PageDown),
        Phrase::PageDown,
    ),
    plain(
        &[KeyCode::PageUp],
        Action::Navigate(Navigation::PageUp),
        Phrase::PageUp,
    ),
    control(
        &[KeyCode::Char('u')],
        Action::Navigate(Navigation::PageUp),
        Phrase::PageUp,
    ),
    plain(
        &[KeyCode::Char('g'), KeyCode::Home],
        Action::Navigate(Navigation::First),
        Phrase::GoToFirst,
    ),
    plain(
        &[KeyCode::Char('G'), KeyCode::End],
        Action::Navigate(Navigation::Last),
        Phrase::GoToLast,
    ),
    plain(&[KeyCode::Enter], Action::Activate, Phrase::Activate),
    plain(
        &[KeyCode::Char('l'), KeyCode::Right],
        Action::Forward,
        Phrase::Forward,
    ),
    plain(
        &[KeyCode::Char('h'), KeyCode::Left, KeyCode::Backspace],
        Action::Back,
        Phrase::Back,
    ),
    plain(&[KeyCode::Char('a')], Action::Add, Phrase::AddSelection),
    plain(&[KeyCode::Char('A')], Action::AddAll, Phrase::AddAll),
    plain(
        &[KeyCode::Char('d'), KeyCode::Delete],
        Action::Delete,
        Phrase::DeleteSelection,
    ),
    plain(
        &[KeyCode::Char('J')],
        Action::MoveSong(MoveDirection::Down),
        Phrase::MoveSong,
    ),
    plain(
        &[KeyCode::Char('K')],
        Action::MoveSong(MoveDirection::Up),
        Phrase::MoveSong,
    ),
    plain(
        &[KeyCode::Char('P')],
        Action::AddToPlaylist,
        Phrase::AddToPlaylist,
    ),
    plain(
        &[KeyCode::Char('p'), KeyCode::Char(' ')],
        Action::TogglePause,
        Phrase::TogglePause,
    ),
    plain(&[KeyCode::Char('s')], Action::Stop, Phrase::Stop),
    plain(&[KeyCode::Char('>')], Action::NextSong, Phrase::NextSong),
    plain(
        &[KeyCode::Char('<')],
        Action::PreviousSong,
        Phrase::PreviousSong,
    ),
    plain(
        &[KeyCode::Char('+'), KeyCode::Char('=')],
        Action::VolumeUp,
        Phrase::VolumeUp,
    ),
    plain(
        &[KeyCode::Char('-')],
        Action::VolumeDown,
        Phrase::VolumeDown,
    ),
    plain(
        &[KeyCode::Char('f')],
        Action::SeekForward,
        Phrase::SeekForward,
    ),
    plain(
        &[KeyCode::Char('b')],
        Action::SeekBackward,
        Phrase::SeekBackward,
    ),
    plain(
        &[KeyCode::Char('c')],
        Action::ClearQueue,
        Phrase::ClearQueue,
    ),
    plain(&[KeyCode::Char('S')], Action::SaveQueue, Phrase::SaveQueue),
    plain(&[KeyCode::Char('/')], Action::StartSearch, Phrase::Search),
    plain(&[KeyCode::Char('z')], Action::ToggleRandom, Phrase::Random),
    plain(&[KeyCode::Char('r')], Action::ToggleRepeat, Phrase::Repeat),
    plain(
        &[KeyCode::Char('y')],
        Action::CycleSingle,
        Phrase::CycleSingle,
    ),
    plain(
        &[KeyCode::Char('R')],
        Action::ToggleConsume,
        Phrase::Consume,
    ),
    plain(
        &[KeyCode::Char('o')],
        Action::ShowPlaying,
        Phrase::ShowPlaying,
    ),
    plain(&[KeyCode::Char('O')], Action::SortQueue, Phrase::SortQueue),
    plain(
        &[KeyCode::Char('i')],
        Action::ShowSongInfo,
        Phrase::ShowSongInfo,
    ),
    control(
        &[KeyCode::Char('r')],
        Action::RetryLyrics,
        Phrase::RetryLyrics,
    ),
    plain(
        &[KeyCode::Char('u')],
        Action::UpdateDatabase,
        Phrase::UpdateDatabase,
    ),
    plain(
        &[KeyCode::Char('n')],
        Action::NewPartition,
        Phrase::NewPartition,
    ),
    plain(
        &[KeyCode::Char('m')],
        Action::MoveOutput,
        Phrase::MoveOutput,
    ),
    plain(&[KeyCode::Char('t')], Action::NextTheme, Phrase::NextTheme),
];

/// La acción que abre cada vista: la del visualizador además cambia de forma.
pub fn view_action(view: ViewKind) -> Action {
    match view {
        ViewKind::Visualizer => Action::ShowVisualizer,
        other => Action::ShowView(other),
    }
}

/// Shift no se distingue: ya viene codificado en el carácter (`G`, `R`, `?`…).
pub fn action_for(key: KeyEvent) -> Option<Action> {
    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    BINDINGS
        .iter()
        .find(|binding| binding.control == control && binding.keys.contains(&key.code))
        .map(|binding| binding.action)
}

/// Secciones de la ayuda, en el orden en que se muestran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpGroup {
    Navigation,
    Lists,
    Playback,
    Modes,
    Other,
}

impl HelpGroup {
    pub const ALL: [HelpGroup; 5] = [
        HelpGroup::Navigation,
        HelpGroup::Lists,
        HelpGroup::Playback,
        HelpGroup::Modes,
        HelpGroup::Other,
    ];

    pub fn title(self) -> Phrase {
        match self {
            HelpGroup::Navigation => Phrase::HelpNavigation,
            HelpGroup::Lists => Phrase::HelpLists,
            HelpGroup::Playback => Phrase::HelpPlayback,
            HelpGroup::Modes => Phrase::HelpModes,
            HelpGroup::Other => Phrase::HelpOther,
        }
    }

    fn of(action: Action) -> Self {
        match action {
            Action::ShowView(_)
            | Action::ShowVisualizer
            | Action::NextView
            | Action::PreviousView
            | Action::Navigate(_)
            | Action::Forward
            | Action::Back => HelpGroup::Navigation,
            Action::Activate
            | Action::Add
            | Action::AddAll
            | Action::Delete
            | Action::MoveSong(_)
            | Action::AddToPlaylist
            | Action::SortQueue
            | Action::ShowSongInfo
            | Action::ClearQueue
            | Action::SaveQueue
            | Action::ShowPlaying => HelpGroup::Lists,
            Action::TogglePause
            | Action::Stop
            | Action::NextSong
            | Action::PreviousSong
            | Action::SeekForward
            | Action::SeekBackward
            | Action::VolumeUp
            | Action::VolumeDown => HelpGroup::Playback,
            Action::ToggleRandom
            | Action::ToggleRepeat
            | Action::CycleSingle
            | Action::ToggleConsume => HelpGroup::Modes,
            Action::StartSearch
            | Action::RetryLyrics
            | Action::UpdateDatabase
            | Action::NewPartition
            | Action::MoveOutput
            | Action::NextTheme
            | Action::ToggleHelp
            | Action::Quit => HelpGroup::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    pub keys: String,
    pub description: Phrase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    pub group: HelpGroup,
    pub entries: Vec<HelpEntry>,
}

/// Texto de una tecla, con `Ctrl+` si hace falta.
fn key_text(binding: &Binding, code: KeyCode, glyphs: &Glyphs, texts: &dyn Catalog) -> String {
    let prefix = if binding.control { CONTROL_PREFIX } else { "" };
    format!("{prefix}{}", key_name(code, glyphs, texts))
}

/// Texto legible de las teclas de un atajo, para la ayuda.
fn keys_label(binding: &Binding, glyphs: &Glyphs, texts: &dyn Catalog) -> String {
    binding
        .keys
        .iter()
        .map(|code| key_text(binding, *code, glyphs, texts))
        .collect::<Vec<_>>()
        .join(" ")
}

/// La tecla principal de una acción (la primera de la tabla), para sugerirla en pantalla.
pub fn key_label(action: Action, glyphs: &Glyphs, texts: &dyn Catalog) -> Option<String> {
    let binding = BINDINGS.iter().find(|binding| binding.action == action)?;
    let code = binding.keys.first()?;
    Some(key_text(binding, *code, glyphs, texts))
}

/// Una línea por descripción, juntando todas sus teclas, agrupadas por sección y en el
/// orden de la tabla. Las flechas se nombran con los glifos del juego activo.
pub fn help_sections(glyphs: &Glyphs, texts: &dyn Catalog) -> Vec<HelpSection> {
    let mut entries: Vec<(HelpGroup, HelpEntry)> = Vec::new();
    for binding in BINDINGS {
        let keys = keys_label(binding, glyphs, texts);
        match entries
            .iter_mut()
            .find(|(_, entry)| entry.description == binding.description)
        {
            Some((_, entry)) => {
                entry.keys.push(' ');
                entry.keys.push_str(&keys);
            }
            None => entries.push((
                HelpGroup::of(binding.action),
                HelpEntry {
                    keys,
                    description: binding.description,
                },
            )),
        }
    }
    HelpGroup::ALL
        .into_iter()
        .map(|group| HelpSection {
            group,
            entries: entries
                .iter()
                .filter(|(entry_group, _)| *entry_group == group)
                .map(|(_, entry)| entry.clone())
                .collect(),
        })
        .collect()
}

/// Las flechas, con su glifo si el juego lo tiene; si no, con palabras del idioma.
fn key_name(code: KeyCode, glyphs: &Glyphs, texts: &dyn Catalog) -> String {
    let arrow = |pick: fn(&ArrowKeys) -> &'static str, word: Phrase| {
        glyphs
            .arrow_keys
            .as_ref()
            .map_or_else(|| texts.phrase(word), pick)
            .to_owned()
    };
    match code {
        KeyCode::Char(' ') => texts.phrase(Phrase::KeySpace).into(),
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Enter => ENTER_KEY.into(),
        KeyCode::Tab => TAB_KEY.into(),
        KeyCode::BackTab => BACK_TAB_KEY.into(),
        KeyCode::Up => arrow(|arrows| arrows.up, Phrase::KeyUp),
        KeyCode::Down => arrow(|arrows| arrows.down, Phrase::KeyDown),
        KeyCode::Left => arrow(|arrows| arrows.left, Phrase::KeyLeft),
        KeyCode::Right => arrow(|arrows| arrows.right, Phrase::KeyRight),
        KeyCode::PageUp => texts.phrase(Phrase::KeyPageUp).into(),
        KeyCode::PageDown => texts.phrase(Phrase::KeyPageDown).into(),
        KeyCode::Home => texts.phrase(Phrase::KeyHome).into(),
        KeyCode::End => texts.phrase(Phrase::KeyEnd).into(),
        KeyCode::Backspace => texts.phrase(Phrase::KeyBackspace).into(),
        KeyCode::Delete => texts.phrase(Phrase::KeyDelete).into(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{English, Spanish};
    use crate::tui::glyphs::{ASCII, UNICODE};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn maps_plain_and_shifted_keys() {
        assert_eq!(
            action_for(key(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(Action::Navigate(Navigation::Next))
        );
        assert_eq!(
            action_for(key(KeyCode::Char('R'), KeyModifiers::SHIFT)),
            Some(Action::ToggleConsume)
        );
        assert_eq!(
            action_for(key(KeyCode::Char('x'), KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn navigation_moves_by_lines_and_pages() {
        assert_eq!(Navigation::Next.apply(3, 10), 4);
        assert_eq!(Navigation::Previous.apply(0, 10), 0);
        assert_eq!(Navigation::PageDown.apply(3, 10), 13);
        assert_eq!(Navigation::PageUp.apply(3, 10), 0);
        assert_eq!(Navigation::First.apply(7, 10), 0);
        assert_eq!(Navigation::Last.apply(7, 10), usize::MAX);
    }

    #[test]
    fn control_r_retries_lyrics_and_plain_r_toggles_repeat() {
        assert_eq!(
            action_for(key(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Some(Action::RetryLyrics)
        );
        assert_eq!(
            action_for(key(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::ToggleRepeat)
        );
    }

    #[test]
    fn u_updates_the_database_and_control_u_pages_up() {
        assert_eq!(
            action_for(key(KeyCode::Char('u'), KeyModifiers::NONE)),
            Some(Action::UpdateDatabase)
        );
        assert_eq!(
            action_for(key(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Some(Action::Navigate(Navigation::PageUp))
        );
    }

    #[test]
    fn six_opens_the_visualizer_and_cycles_its_styles() {
        assert_eq!(
            action_for(key(KeyCode::Char('6'), KeyModifiers::NONE)),
            Some(Action::ShowVisualizer)
        );
        assert_eq!(view_action(ViewKind::Visualizer), Action::ShowVisualizer);
        assert_eq!(
            view_action(ViewKind::Queue),
            Action::ShowView(ViewKind::Queue)
        );
        for view in ViewKind::ALL {
            assert!(
                key_label(view_action(view), &UNICODE, &English).is_some(),
                "{view:?}"
            );
        }
    }

    #[test]
    fn shifted_j_and_k_move_the_song_and_plain_ones_navigate() {
        assert_eq!(
            action_for(key(KeyCode::Char('J'), KeyModifiers::SHIFT)),
            Some(Action::MoveSong(MoveDirection::Down))
        );
        assert_eq!(
            action_for(key(KeyCode::Char('K'), KeyModifiers::SHIFT)),
            Some(Action::MoveSong(MoveDirection::Up))
        );
        assert_eq!(
            action_for(key(KeyCode::Char('k'), KeyModifiers::NONE)),
            Some(Action::Navigate(Navigation::Previous))
        );
        assert_eq!(
            action_for(key(KeyCode::Char('P'), KeyModifiers::SHIFT)),
            Some(Action::AddToPlaylist)
        );
        let moving = all_entries(&UNICODE)
            .into_iter()
            .find(|entry| entry.description == Phrase::MoveSong)
            .unwrap();
        assert_eq!(moving.keys, "J K");
    }

    #[test]
    fn control_changes_meaning() {
        assert_eq!(
            action_for(key(KeyCode::Char('c'), KeyModifiers::NONE)),
            Some(Action::ClearQueue)
        );
        assert_eq!(
            action_for(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Action::Quit)
        );
    }

    #[test]
    fn every_key_has_a_single_meaning() {
        let mut seen = Vec::new();
        for binding in BINDINGS {
            for code in binding.keys {
                let combination = (binding.control, *code);
                assert!(
                    !seen.contains(&combination),
                    "tecla repetida: {combination:?}"
                );
                seen.push(combination);
            }
        }
    }

    fn all_entries(glyphs: &Glyphs) -> Vec<HelpEntry> {
        help_sections(glyphs, &Spanish)
            .into_iter()
            .flat_map(|section| section.entries)
            .collect()
    }

    #[test]
    fn eight_opens_the_outputs_and_n_m_manage_them() {
        let press =
            |character| action_for(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        assert_eq!(press('8'), Some(Action::ShowView(ViewKind::Outputs)));
        assert_eq!(press('n'), Some(Action::NewPartition));
        assert_eq!(press('m'), Some(Action::MoveOutput));
    }

    #[test]
    fn shifted_o_sorts_the_queue_and_plain_o_shows_the_playing_song() {
        assert_eq!(
            action_for(key(KeyCode::Char('O'), KeyModifiers::SHIFT)),
            Some(Action::SortQueue)
        );
        assert_eq!(
            action_for(key(KeyCode::Char('o'), KeyModifiers::NONE)),
            Some(Action::ShowPlaying)
        );
    }

    #[test]
    fn i_shows_the_song_information() {
        assert_eq!(
            action_for(key(KeyCode::Char('i'), KeyModifiers::NONE)),
            Some(Action::ShowSongInfo)
        );
    }

    #[test]
    fn tab_and_shift_tab_share_a_help_line() {
        let views = all_entries(&UNICODE)
            .into_iter()
            .find(|entry| entry.description == Phrase::CycleViews)
            .unwrap();
        assert_eq!(views.keys, "Tab Shift+Tab");
    }

    #[test]
    fn t_cycles_the_color_theme() {
        assert_eq!(
            action_for(key(KeyCode::Char('t'), KeyModifiers::NONE)),
            Some(Action::NextTheme)
        );
        assert_eq!(
            key_label(Action::NextTheme, &ASCII, &Spanish).as_deref(),
            Some("t")
        );
    }

    #[test]
    fn help_groups_keys_by_description() {
        let entries = all_entries(&UNICODE);
        let views = entries
            .iter()
            .find(|entry| entry.description == Phrase::GoToView)
            .unwrap();
        assert_eq!(views.keys, "1 2 3 4 5 7 8");
        let quit = entries
            .iter()
            .find(|entry| entry.description == Phrase::Quit)
            .unwrap();
        assert_eq!(quit.keys, "q Ctrl+c");
        let mut descriptions: Vec<_> = Vec::new();
        for binding in BINDINGS {
            if !descriptions.contains(&binding.description) {
                descriptions.push(binding.description);
            }
        }
        assert_eq!(entries.len(), descriptions.len());
    }

    #[test]
    fn help_sections_follow_their_order() {
        let sections = help_sections(&UNICODE, &English);
        let groups: Vec<_> = sections.iter().map(|section| section.group).collect();
        assert_eq!(groups, HelpGroup::ALL);
        assert!(sections.iter().all(|section| !section.entries.is_empty()));
    }

    #[test]
    fn key_label_is_the_first_key_of_the_action() {
        assert_eq!(
            key_label(Action::Activate, &UNICODE, &English).as_deref(),
            Some("Enter")
        );
        assert_eq!(
            key_label(Action::TogglePause, &UNICODE, &English).as_deref(),
            Some("p")
        );
        assert_eq!(
            key_label(Action::ToggleHelp, &ASCII, &English).as_deref(),
            Some("?")
        );
    }

    #[test]
    fn labels_are_readable() {
        assert_eq!(keys_label(&BINDINGS[1], &UNICODE, &Spanish), "Ctrl+c");
        let pause = BINDINGS
            .iter()
            .find(|binding| binding.action == Action::TogglePause)
            .unwrap();
        assert_eq!(keys_label(pause, &UNICODE, &Spanish), "p Espacio");
        assert_eq!(keys_label(pause, &UNICODE, &English), "p Space");
    }

    #[test]
    fn arrow_names_follow_the_glyph_set() {
        let down = BINDINGS
            .iter()
            .find(|binding| binding.action == Action::Navigate(Navigation::Next))
            .unwrap();
        let arrows = UNICODE.arrow_keys.unwrap();
        assert_eq!(
            keys_label(down, &UNICODE, &English),
            format!("j {}", arrows.down)
        );
        assert_eq!(keys_label(down, &ASCII, &English), "j Down");
        assert_eq!(keys_label(down, &ASCII, &Spanish), "j Abajo");
        let unicode_arrows = [arrows.up, arrows.down, arrows.left, arrows.right];
        for entry in all_entries(&ASCII) {
            assert!(
                !unicode_arrows
                    .iter()
                    .any(|arrow| entry.keys.contains(arrow))
            );
        }
    }
}
