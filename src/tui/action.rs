//! Mapa de atajos: una sola tabla de la que salen la traducción de teclas y la ayuda.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::glyphs::Glyphs;
use super::state::ViewKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    Next,
    Previous,
    PageDown,
    PageUp,
    First,
    Last,
}

/// Acciones del usuario. Las de vista (`Activate`, `Add`, `Delete`…) las interpreta
/// la vista activa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ShowView(ViewKind),
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
}

pub struct Binding {
    pub keys: &'static [KeyCode],
    pub control: bool,
    pub action: Action,
    pub description: &'static str,
}

const fn plain(keys: &'static [KeyCode], action: Action, description: &'static str) -> Binding {
    Binding {
        keys,
        control: false,
        action,
        description,
    }
}

const fn control(keys: &'static [KeyCode], action: Action, description: &'static str) -> Binding {
    Binding {
        keys,
        control: true,
        action,
        description,
    }
}

/// Las cinco vistas comparten descripción: la ayuda las muestra en una sola línea.
const GO_TO_VIEW: &str = "Ir a la vista";

pub const BINDINGS: &[Binding] = &[
    plain(&[KeyCode::Char('q')], Action::Quit, "Salir"),
    control(&[KeyCode::Char('c')], Action::Quit, "Salir"),
    plain(
        &[KeyCode::Char('?')],
        Action::ToggleHelp,
        "Mostrar u ocultar la ayuda",
    ),
    plain(
        &[KeyCode::Char('1')],
        Action::ShowView(ViewKind::Queue),
        GO_TO_VIEW,
    ),
    plain(
        &[KeyCode::Char('2')],
        Action::ShowView(ViewKind::Library),
        GO_TO_VIEW,
    ),
    plain(
        &[KeyCode::Char('3')],
        Action::ShowView(ViewKind::Browse),
        GO_TO_VIEW,
    ),
    plain(
        &[KeyCode::Char('4')],
        Action::ShowView(ViewKind::Search),
        GO_TO_VIEW,
    ),
    plain(
        &[KeyCode::Char('5')],
        Action::ShowView(ViewKind::Playlists),
        GO_TO_VIEW,
    ),
    plain(
        &[KeyCode::Char('6')],
        Action::ShowView(ViewKind::Spectrum),
        GO_TO_VIEW,
    ),
    plain(&[KeyCode::Tab], Action::NextView, "Vista siguiente"),
    plain(&[KeyCode::BackTab], Action::PreviousView, "Vista anterior"),
    plain(
        &[KeyCode::Char('j'), KeyCode::Down],
        Action::Navigate(Navigation::Next),
        "Bajar",
    ),
    plain(
        &[KeyCode::Char('k'), KeyCode::Up],
        Action::Navigate(Navigation::Previous),
        "Subir",
    ),
    plain(
        &[KeyCode::PageDown],
        Action::Navigate(Navigation::PageDown),
        "Bajar una página",
    ),
    control(
        &[KeyCode::Char('d')],
        Action::Navigate(Navigation::PageDown),
        "Bajar una página",
    ),
    plain(
        &[KeyCode::PageUp],
        Action::Navigate(Navigation::PageUp),
        "Subir una página",
    ),
    control(
        &[KeyCode::Char('u')],
        Action::Navigate(Navigation::PageUp),
        "Subir una página",
    ),
    plain(
        &[KeyCode::Char('g'), KeyCode::Home],
        Action::Navigate(Navigation::First),
        "Ir al principio",
    ),
    plain(
        &[KeyCode::Char('G'), KeyCode::End],
        Action::Navigate(Navigation::Last),
        "Ir al final",
    ),
    plain(
        &[KeyCode::Enter],
        Action::Activate,
        "Reproducir / entrar / cargar",
    ),
    plain(
        &[KeyCode::Char('l'), KeyCode::Right],
        Action::Forward,
        "Entrar / columna siguiente",
    ),
    plain(
        &[KeyCode::Char('h'), KeyCode::Left, KeyCode::Backspace],
        Action::Back,
        "Volver / columna anterior",
    ),
    plain(
        &[KeyCode::Char('a')],
        Action::Add,
        "Agregar (playlist: cargar)",
    ),
    plain(&[KeyCode::Char('A')], Action::AddAll, "Agregar todos"),
    plain(
        &[KeyCode::Char('d'), KeyCode::Delete],
        Action::Delete,
        "Borrar (playlist: confirma)",
    ),
    plain(
        &[KeyCode::Char('p'), KeyCode::Char(' ')],
        Action::TogglePause,
        "Pausa / reanudar",
    ),
    plain(&[KeyCode::Char('s')], Action::Stop, "Detener"),
    plain(&[KeyCode::Char('>')], Action::NextSong, "Tema siguiente"),
    plain(&[KeyCode::Char('<')], Action::PreviousSong, "Tema anterior"),
    plain(
        &[KeyCode::Char('+'), KeyCode::Char('=')],
        Action::VolumeUp,
        "Subir volumen",
    ),
    plain(&[KeyCode::Char('-')], Action::VolumeDown, "Bajar volumen"),
    plain(&[KeyCode::Char('f')], Action::SeekForward, "Adelantar"),
    plain(&[KeyCode::Char('b')], Action::SeekBackward, "Retroceder"),
    plain(
        &[KeyCode::Char('c')],
        Action::ClearQueue,
        "Vaciar la cola (confirma)",
    ),
    plain(
        &[KeyCode::Char('S')],
        Action::SaveQueue,
        "Guardar la cola como playlist",
    ),
    plain(&[KeyCode::Char('/')], Action::StartSearch, "Buscar"),
    plain(&[KeyCode::Char('z')], Action::ToggleRandom, "Aleatorio"),
    plain(&[KeyCode::Char('r')], Action::ToggleRepeat, "Repetir"),
    plain(
        &[KeyCode::Char('y')],
        Action::CycleSingle,
        "Single: no, sí, una vez",
    ),
    plain(&[KeyCode::Char('R')], Action::ToggleConsume, "Consumir"),
    plain(
        &[KeyCode::Char('o')],
        Action::ShowPlaying,
        "Ir al tema que suena",
    ),
];

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

    pub fn title(self) -> &'static str {
        match self {
            HelpGroup::Navigation => "Navegación",
            HelpGroup::Lists => "Cola y listas",
            HelpGroup::Playback => "Reproducción",
            HelpGroup::Modes => "Modos",
            HelpGroup::Other => "Otros",
        }
    }

    fn of(action: Action) -> Self {
        match action {
            Action::ShowView(_)
            | Action::NextView
            | Action::PreviousView
            | Action::Navigate(_)
            | Action::Forward
            | Action::Back => HelpGroup::Navigation,
            Action::Activate
            | Action::Add
            | Action::AddAll
            | Action::Delete
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
            Action::StartSearch | Action::ToggleHelp | Action::Quit => HelpGroup::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    pub keys: String,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    pub group: HelpGroup,
    pub entries: Vec<HelpEntry>,
}

/// Texto de una tecla, con `Ctrl+` si hace falta.
fn key_text(binding: &Binding, code: KeyCode, glyphs: &Glyphs) -> String {
    let prefix = if binding.control { "Ctrl+" } else { "" };
    format!("{prefix}{}", key_name(code, glyphs))
}

/// Texto legible de las teclas de un atajo, para la ayuda.
fn keys_label(binding: &Binding, glyphs: &Glyphs) -> String {
    binding
        .keys
        .iter()
        .map(|code| key_text(binding, *code, glyphs))
        .collect::<Vec<_>>()
        .join(" ")
}

/// La tecla principal de una acción (la primera de la tabla), para sugerirla en pantalla.
pub fn key_label(action: Action, glyphs: &Glyphs) -> Option<String> {
    let binding = BINDINGS.iter().find(|binding| binding.action == action)?;
    let code = binding.keys.first()?;
    Some(key_text(binding, *code, glyphs))
}

/// Una línea por descripción, juntando todas sus teclas, agrupadas por sección y en el
/// orden de la tabla. Las flechas se nombran con los glifos del juego activo.
pub fn help_sections(glyphs: &Glyphs) -> Vec<HelpSection> {
    let mut entries: Vec<(HelpGroup, HelpEntry)> = Vec::new();
    for binding in BINDINGS {
        let keys = keys_label(binding, glyphs);
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

fn key_name(code: KeyCode, glyphs: &Glyphs) -> String {
    match code {
        KeyCode::Char(' ') => "Espacio".into(),
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::BackTab => "Shift+Tab".into(),
        KeyCode::Up => glyphs.key_up.into(),
        KeyCode::Down => glyphs.key_down.into(),
        KeyCode::Left => glyphs.key_left.into(),
        KeyCode::Right => glyphs.key_right.into(),
        KeyCode::PageUp => "RePág".into(),
        KeyCode::PageDown => "AvPág".into(),
        KeyCode::Home => "Inicio".into(),
        KeyCode::End => "Fin".into(),
        KeyCode::Backspace => "Retroceso".into(),
        KeyCode::Delete => "Supr".into(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        help_sections(glyphs)
            .into_iter()
            .flat_map(|section| section.entries)
            .collect()
    }

    #[test]
    fn help_groups_keys_by_description() {
        let entries = all_entries(&UNICODE);
        let views = entries
            .iter()
            .find(|entry| entry.description == GO_TO_VIEW)
            .unwrap();
        assert_eq!(views.keys, "1 2 3 4 5 6");
        let quit = entries
            .iter()
            .find(|entry| entry.description == "Salir")
            .unwrap();
        assert_eq!(quit.keys, "q Ctrl+c");
        let mut descriptions: Vec<_> = BINDINGS.iter().map(|binding| binding.description).collect();
        descriptions.sort_unstable();
        descriptions.dedup();
        assert_eq!(entries.len(), descriptions.len());
    }

    #[test]
    fn help_sections_follow_their_order() {
        let sections = help_sections(&UNICODE);
        let groups: Vec<_> = sections.iter().map(|section| section.group).collect();
        assert_eq!(groups, HelpGroup::ALL);
        assert!(sections.iter().all(|section| !section.entries.is_empty()));
    }

    #[test]
    fn key_label_is_the_first_key_of_the_action() {
        assert_eq!(
            key_label(Action::Activate, &UNICODE).as_deref(),
            Some("Enter")
        );
        assert_eq!(
            key_label(Action::TogglePause, &UNICODE).as_deref(),
            Some("p")
        );
        assert_eq!(key_label(Action::ToggleHelp, &ASCII).as_deref(), Some("?"));
    }

    #[test]
    fn labels_are_readable() {
        assert_eq!(keys_label(&BINDINGS[1], &UNICODE), "Ctrl+c");
        let pause = BINDINGS
            .iter()
            .find(|binding| binding.action == Action::TogglePause)
            .unwrap();
        assert_eq!(keys_label(pause, &UNICODE), "p Espacio");
    }

    #[test]
    fn arrow_names_follow_the_glyph_set() {
        let down = BINDINGS
            .iter()
            .find(|binding| binding.action == Action::Navigate(Navigation::Next))
            .unwrap();
        assert_eq!(
            keys_label(down, &UNICODE),
            format!("j {}", UNICODE.key_down)
        );
        assert_eq!(keys_label(down, &ASCII), format!("j {}", ASCII.key_down));
        let unicode_arrows = [
            UNICODE.key_up,
            UNICODE.key_down,
            UNICODE.key_left,
            UNICODE.key_right,
        ];
        for entry in all_entries(&ASCII) {
            assert!(
                !unicode_arrows
                    .iter()
                    .any(|arrow| entry.keys.contains(arrow))
            );
        }
    }
}
