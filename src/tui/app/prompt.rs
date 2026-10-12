//! Manejo de la línea de entrada y de las confirmaciones.

use crossterm::event::KeyEvent;

use super::{App, Backend};
use crate::domain::PlaylistName;
use crate::i18n::Phrase;
use crate::tui::prompt::{Confirmation, Prompt, PromptKind, PromptOutcome};

impl<B: Backend> App<B> {
    pub(super) async fn handle_prompt_key(&mut self, key: KeyEvent) {
        let Some(prompt) = self.state.prompt.as_mut() else {
            return;
        };
        match prompt.handle_key(key) {
            PromptOutcome::Editing => {}
            PromptOutcome::NextField => self.state.search.cycle_field(),
            PromptOutcome::Cancelled => self.state.prompt = None,
            PromptOutcome::Submitted => {
                if let Some(prompt) = self.state.prompt.take() {
                    self.submit_prompt(prompt).await;
                }
            }
        }
    }

    async fn submit_prompt(&mut self, mut prompt: Prompt) {
        match &prompt.kind {
            PromptKind::Search => self.submit_search(&prompt.input).await,
            // Validación en el borde: si el nombre no sirve, el diálogo sigue abierto.
            PromptKind::SavePlaylist => match PlaylistName::new(&prompt.input) {
                Ok(name) => self.save_queue(&name).await,
                Err(error) => {
                    prompt.error = Some(error);
                    self.state.prompt = Some(prompt);
                }
            },
            PromptKind::Confirm(Confirmation::ClearQueue) => self.clear_queue().await,
            PromptKind::Confirm(Confirmation::RemovePlaylist(name)) => {
                self.remove_playlist(name).await;
            }
        }
    }

    async fn clear_queue(&mut self) {
        let result = self.services.queue.clear().await;
        if self.report(result).is_some() {
            self.notify_info(self.phrase(Phrase::QueueCleared));
        }
    }
}
