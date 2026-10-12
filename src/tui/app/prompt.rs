//! Manejo de la línea de entrada y de las confirmaciones.

use crossterm::event::KeyEvent;

use super::{App, Backend};
use crate::domain::{InputError, PartitionName, PlaylistName};
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

    async fn submit_prompt(&mut self, prompt: Prompt) {
        match &prompt.kind {
            PromptKind::Search => self.submit_search(&prompt.input).await,
            PromptKind::SavePlaylist | PromptKind::NewPlaylist(_) => {
                self.submit_playlist_name(prompt).await;
            }
            PromptKind::NewPartition => {
                if let Some((name, _)) = self.validate(prompt, PartitionName::new) {
                    self.create_partition(&name).await;
                }
            }
            PromptKind::Confirm(Confirmation::ClearQueue) => self.clear_queue().await,
            PromptKind::Confirm(Confirmation::RemovePlaylist(name)) => {
                self.remove_playlist(name).await;
            }
            PromptKind::Confirm(Confirmation::RemovePartition(partition)) => {
                self.remove_partition(partition).await;
            }
        }
    }

    async fn submit_playlist_name(&mut self, prompt: Prompt) {
        let Some((name, prompt)) = self.validate(prompt, PlaylistName::new) else {
            return;
        };
        match &prompt.kind {
            PromptKind::SavePlaylist => self.save_queue(&name).await,
            PromptKind::NewPlaylist(addition) => self.add_to_playlist(&name, addition).await,
            PromptKind::Search | PromptKind::NewPartition | PromptKind::Confirm(_) => {}
        }
    }

    /// Validación en el borde: si la entrada no sirve, el diálogo sigue abierto con el
    /// error. Si sirve, devuelve el valor y el diálogo (que dice para qué era).
    fn validate<T>(
        &mut self,
        mut prompt: Prompt,
        parse: impl Fn(&str) -> Result<T, InputError>,
    ) -> Option<(T, Prompt)> {
        match parse(&prompt.input) {
            Ok(value) => Some((value, prompt)),
            Err(error) => {
                prompt.error = Some(error);
                self.state.prompt = Some(prompt);
                None
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
