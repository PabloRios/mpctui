//! Acciones de la vista de búsqueda.

use super::{App, Backend};
use crate::domain::SearchText;
use crate::tui::action::Action;
use crate::tui::list::SelectionPolicy;
use crate::tui::prompt::{Prompt, PromptKind};
use crate::tui::state::ViewKind;

impl<B: Backend> App<B> {
    pub(super) async fn search_action(&mut self, action: Action) {
        match action {
            Action::Navigate(navigation) => {
                self.state.search.results.navigate(navigation);
            }
            Action::Activate | Action::Add => {
                let song = self.state.search.results.selected().cloned();
                self.add_songs(song.into_iter().collect()).await;
            }
            Action::AddAll => {
                let songs = self.state.search.results.items().to_vec();
                self.add_songs(songs).await;
            }
            _ => {}
        }
    }

    /// Abre la vista de búsqueda con el texto anterior para editarlo.
    pub(super) async fn start_search(&mut self) {
        self.show_view(ViewKind::Search).await;
        let previous = self
            .state
            .search
            .text
            .as_ref()
            .map(|text| text.as_str().to_owned())
            .unwrap_or_default();
        self.state.prompt = Some(Prompt::text(PromptKind::Search, previous));
    }

    /// Valida el texto en el borde antes de consultar al servidor.
    pub(super) async fn submit_search(&mut self, input: &str) {
        let text = match SearchText::new(input) {
            Ok(text) => text,
            Err(error) => return self.notify_error(error.to_string()),
        };
        let field = self.state.search.field;
        let result = self.services.library.search(field, &text).await;
        if let Some(songs) = self.report(result) {
            self.notify_info(format!("{} resultados", songs.len()));
            self.state
                .search
                .results
                .set_items(songs, SelectionPolicy::Reset);
            self.state.search.text = Some(text);
        }
    }
}
