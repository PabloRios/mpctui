//! Acciones de la vista Salidas: con foco en las particiones se elige cuál usa mpctui y
//! se crean y se borran; con foco en las salidas se activan, se desactivan y se mueven a
//! otra partición.

use super::{App, Backend};
use crate::domain::{Output, Partition, PartitionName};
use crate::i18n::{Message, Phrase};
use crate::tui::action::Action;
use crate::tui::list::SelectionPolicy;
use crate::tui::picker::{PartitionPicker, Picker};
use crate::tui::prompt::{Confirmation, Prompt, PromptKind};
use crate::tui::state::OutputsColumn;

impl<B: Backend> App<B> {
    pub(super) async fn outputs_action(&mut self, action: Action) {
        match self.state.outputs.focus {
            OutputsColumn::Partitions => self.partitions_action(action).await,
            OutputsColumn::Outputs => self.output_list_action(action).await,
        }
    }

    /// Se llama también después de cada acción propia: el aviso de `idle` de un cambio
    /// en otra partición puede no llegar a la conexión que vigila.
    pub(super) async fn reload_outputs(&mut self) {
        let result = self.services.outputs.overview().await;
        if let Some(partitions) = self.report(result) {
            self.state.outputs.set_partitions(partitions);
        }
    }

    /// Recibe el nombre ya validado por el diálogo.
    pub(super) async fn create_partition(&mut self, name: &PartitionName) {
        let result = self.services.outputs.create(name).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::PartitionCreated(name.as_str())));
        }
        self.reload_outputs().await;
    }

    pub(super) async fn remove_partition(&mut self, partition: &Partition) {
        let result = self.services.outputs.delete(partition).await;
        if self.report(result).is_some() {
            let name = partition.name.as_str();
            self.notify_info(self.message(Message::PartitionDeleted(name)));
        }
        self.reload_outputs().await;
    }

    pub(super) async fn move_output(&mut self, output: &Output, to: &PartitionName) {
        let result = self.services.outputs.move_output(output, to).await;
        if self.report(result).is_some() {
            self.notify_info(self.message(Message::OutputMoved {
                output: &output.name,
                partition: to.as_str(),
            }));
        }
        self.reload_outputs().await;
    }

    async fn partitions_action(&mut self, action: Action) {
        let view = &mut self.state.outputs;
        match action {
            Action::Navigate(navigation) => {
                if view.partitions.navigate(navigation) {
                    view.show_selected_outputs(SelectionPolicy::Reset);
                }
            }
            Action::Forward if !view.outputs.items().is_empty() => {
                view.focus = OutputsColumn::Outputs;
            }
            Action::Activate => self.use_selected_partition().await,
            Action::NewPartition => self.ask_partition_name(),
            Action::Delete => self.confirm_partition_removal(),
            _ => {}
        }
    }

    async fn output_list_action(&mut self, action: Action) {
        match action {
            Action::Navigate(navigation) => {
                self.state.outputs.outputs.navigate(navigation);
            }
            Action::Back => self.state.outputs.focus = OutputsColumn::Partitions,
            Action::Activate => self.toggle_selected_output().await,
            Action::MoveOutput => self.open_partition_picker(),
            Action::NewPartition => self.ask_partition_name(),
            _ => {}
        }
    }

    fn ask_partition_name(&mut self) {
        self.state.prompt = Some(Prompt::text(PromptKind::NewPartition, String::new()));
    }

    /// Todas las conexiones que la siguen entran a la nueva con su próximo comando; el
    /// vigilante se relanza porque `idle` sigue esperando en la anterior. Después se
    /// relee todo: la cola y la reproducción son otras.
    async fn use_selected_partition(&mut self) {
        let Some(partition) = self.state.outputs.selected_partition() else {
            return;
        };
        let name = partition.name.clone();
        if name == self.services.outputs.active() {
            return;
        }
        self.services.outputs.use_partition(name.clone());
        self.state.partition = name.clone();
        self.watcher = Some(self.services.watcher.launch());
        self.resynchronize().await;
        self.notify_info(self.message(Message::UsingPartition(name.as_str())));
    }

    /// `default` y la partición en uso no se pueden borrar: se avisa sin preguntar.
    fn confirm_partition_removal(&mut self) {
        let Some(partition) = self.state.outputs.selected_partition().cloned() else {
            return;
        };
        if partition.name.is_default() {
            return self.notify_error(self.phrase(Phrase::DefaultPartitionUndeletable));
        }
        if partition.name == self.services.outputs.active() {
            return self.notify_error(self.phrase(Phrase::ActivePartitionUndeletable));
        }
        let confirmation = Confirmation::RemovePartition(partition);
        self.state.prompt = Some(Prompt::confirm(confirmation));
    }

    async fn toggle_selected_output(&mut self) {
        let Some((partition, output)) = self.selected_output() else {
            return;
        };
        let result = self.services.outputs.toggle(&partition, &output).await;
        if let Some(enabled) = self.report(result) {
            self.notify_info(self.message(Message::OutputToggled {
                output: &output.name,
                enabled,
            }));
        }
        self.reload_outputs().await;
    }

    /// Los destinos son las demás particiones; si no hay, se sugiere crear una.
    fn open_partition_picker(&mut self) {
        let Some((current, output)) = self.selected_output() else {
            return;
        };
        let destinations: Vec<PartitionName> = self
            .state
            .outputs
            .partitions
            .items()
            .iter()
            .map(|partition| partition.name.clone())
            .filter(|name| *name != current)
            .collect();
        if destinations.is_empty() {
            let key = self.key(Action::NewPartition);
            return self.notify_info(self.message(Message::NoOtherPartition { key: &key }));
        }
        let picker = PartitionPicker::new(output, destinations);
        self.state.picker = Some(Picker::Partition(picker));
    }

    /// La partición elegida y la salida seleccionada en ella.
    fn selected_output(&self) -> Option<(PartitionName, Output)> {
        let view = &self.state.outputs;
        let partition = view.selected_partition()?.name.clone();
        Some((partition, view.outputs.selected()?.clone()))
    }
}
