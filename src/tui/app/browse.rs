//! Acciones del navegador por directorios.

use super::{App, Backend};
use crate::domain::Entry;
use crate::i18n::Message;
use crate::tui::action::Action;
use crate::tui::list::SelectionPolicy;

impl<B: Backend> App<B> {
    pub(super) async fn browse_action(&mut self, action: Action) {
        match action {
            Action::Navigate(navigation) => {
                self.state.browse.entries.navigate(navigation);
            }
            Action::Activate => self.activate_entry().await,
            Action::Forward => self.enter_selected_directory().await,
            Action::Back => self.go_to_parent_directory().await,
            Action::Add => self.add_selected_entry().await,
            _ => {}
        }
    }

    pub(super) async fn open_directory(&mut self, path: String, policy: SelectionPolicy) {
        let result = self.services.library.browse(&path).await;
        if let Some(entries) = self.report(result) {
            self.state.browse.entries.set_items(entries, policy);
            self.state.browse.path = path;
        }
    }

    /// Enter: entra a un directorio; sobre un archivo o playlist, lo agrega.
    async fn activate_entry(&mut self) {
        match self.state.browse.entries.selected() {
            Some(Entry::Directory { .. }) => self.enter_selected_directory().await,
            Some(_) => self.add_selected_entry().await,
            None => {}
        }
    }

    async fn enter_selected_directory(&mut self) {
        if let Some(Entry::Directory { path }) = self.state.browse.entries.selected() {
            let path = path.clone();
            self.open_directory(path, SelectionPolicy::Reset).await;
        }
    }

    /// Sube un nivel y deja seleccionado el directorio del que se salió.
    async fn go_to_parent_directory(&mut self) {
        let Some(parent) = self.state.browse.parent_path() else {
            return;
        };
        let previous = self.state.browse.path.clone();
        self.open_directory(parent, SelectionPolicy::Reset).await;
        self.state
            .browse
            .entries
            .select_where(|entry| entry.path() == previous);
    }

    async fn add_selected_entry(&mut self) {
        let Some(path) = self
            .state
            .browse
            .entries
            .selected()
            .map(|entry| entry.path().to_owned())
        else {
            return;
        };
        let result = self.services.queue.add_uri(&path).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::AddedToQueue(&path)));
        }
    }
}
