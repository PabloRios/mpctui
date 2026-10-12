//! Línea de entrada: texto libre (búsqueda, nombre de playlist) o confirmación s/n.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::PlaylistName;

/// Teclas que confirman (español e inglés).
pub const CONFIRM_KEYS: [char; 4] = ['s', 'S', 'y', 'Y'];
pub const DECLINE_KEYS: [char; 2] = ['n', 'N'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmation {
    ClearQueue,
    RemovePlaylist(PlaylistName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Search,
    SavePlaylist,
    Confirm(Confirmation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptOutcome {
    Editing,
    Submitted,
    Cancelled,
    /// Tab en la búsqueda: cambiar el campo.
    NextField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    pub input: String,
    /// Por qué no se aceptó la última entrada; el diálogo sigue abierto.
    pub error: Option<String>,
}

impl Prompt {
    pub fn text(kind: PromptKind, initial: String) -> Self {
        Self {
            kind,
            input: initial,
            error: None,
        }
    }

    pub fn confirm(confirmation: Confirmation) -> Self {
        Self {
            kind: PromptKind::Confirm(confirmation),
            input: String::new(),
            error: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        match self.kind {
            PromptKind::Confirm(_) => confirmation_outcome(key.code),
            PromptKind::Search | PromptKind::SavePlaylist => self.edit(key),
        }
    }

    fn edit(&mut self, key: KeyEvent) -> PromptOutcome {
        if is_shortcut(key) {
            return PromptOutcome::Editing;
        }
        match key.code {
            KeyCode::Enter => PromptOutcome::Submitted,
            KeyCode::Esc => PromptOutcome::Cancelled,
            KeyCode::Tab if self.kind == PromptKind::Search => PromptOutcome::NextField,
            KeyCode::Backspace => {
                self.input.pop();
                self.error = None;
                PromptOutcome::Editing
            }
            KeyCode::Char(character) => {
                self.input.push(character);
                self.error = None;
                PromptOutcome::Editing
            }
            _ => PromptOutcome::Editing,
        }
    }
}

/// Ctrl o Alt con una tecla es un atajo, no texto.
fn is_shortcut(key: KeyEvent) -> bool {
    key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}

fn confirmation_outcome(code: KeyCode) -> PromptOutcome {
    match code {
        KeyCode::Char(character) if CONFIRM_KEYS.contains(&character) => PromptOutcome::Submitted,
        KeyCode::Char(character) if DECLINE_KEYS.contains(&character) => PromptOutcome::Cancelled,
        KeyCode::Esc => PromptOutcome::Cancelled,
        _ => PromptOutcome::Editing,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;

    fn press(prompt: &mut Prompt, code: KeyCode) -> PromptOutcome {
        prompt.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn text_prompt_edits_and_submits() {
        let mut prompt = Prompt::text(PromptKind::Search, String::new());
        press(&mut prompt, KeyCode::Char('l'));
        press(&mut prompt, KeyCode::Char('o'));
        press(&mut prompt, KeyCode::Backspace);
        assert_eq!(prompt.input, "l");
        assert_eq!(press(&mut prompt, KeyCode::Tab), PromptOutcome::NextField);
        assert_eq!(press(&mut prompt, KeyCode::Enter), PromptOutcome::Submitted);
    }

    #[test]
    fn control_combinations_are_not_text() {
        let mut prompt = Prompt::text(PromptKind::Search, "ab".into());
        prompt.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(prompt.input, "ab");
    }

    #[test]
    fn editing_clears_the_previous_error() {
        let mut prompt = Prompt::text(PromptKind::SavePlaylist, "a/b".into());
        prompt.error = Some("inválido".into());
        press(&mut prompt, KeyCode::Backspace);
        assert_eq!(prompt.error, None);
    }

    #[test]
    fn tab_only_changes_field_in_search() {
        let mut prompt = Prompt::text(PromptKind::SavePlaylist, String::new());
        assert_eq!(press(&mut prompt, KeyCode::Tab), PromptOutcome::Editing);
        assert_eq!(press(&mut prompt, KeyCode::Esc), PromptOutcome::Cancelled);
    }

    #[test]
    fn confirmation_accepts_yes_and_no() {
        let mut prompt = Prompt::confirm(Confirmation::ClearQueue);
        assert_eq!(
            press(&mut prompt, KeyCode::Char('x')),
            PromptOutcome::Editing
        );
        assert_eq!(
            press(&mut prompt, KeyCode::Char('s')),
            PromptOutcome::Submitted
        );
        assert_eq!(
            press(&mut prompt, KeyCode::Char('n')),
            PromptOutcome::Cancelled
        );
    }
}
