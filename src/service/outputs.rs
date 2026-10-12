//! Particiones de MPD y sus salidas de audio.

use crate::domain::{Output, Partition, PartitionName};
use crate::repository::{OutputRepository, Result};

pub struct OutputService<R> {
    repository: R,
}

impl<R: OutputRepository + Send> OutputService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Cada partición, en el orden del servidor, con las salidas que le pertenecen.
    pub async fn overview(&mut self) -> Result<Vec<Partition>> {
        let names = self.repository.partitions().await?;
        let mut partitions = Vec::with_capacity(names.len());
        for name in names {
            let outputs = self.repository.outputs(&name).await?;
            partitions.push(Partition { name, outputs });
        }
        Ok(partitions)
    }

    pub async fn create(&mut self, name: &PartitionName) -> Result<()> {
        self.repository.create_partition(name).await
    }

    /// MPD no borra una partición con salidas: primero vuelven a `default`.
    pub async fn delete(&mut self, partition: &Partition) -> Result<()> {
        let default = PartitionName::default_partition();
        for output in &partition.outputs {
            self.repository.move_output(&output.name, &default).await?;
        }
        self.repository.delete_partition(&partition.name).await
    }

    pub async fn move_output(&mut self, output: &Output, to: &PartitionName) -> Result<()> {
        self.repository.move_output(&output.name, to).await
    }

    /// Activa la salida si estaba desactivada y viceversa. Devuelve el estado nuevo.
    pub async fn toggle(&mut self, partition: &PartitionName, output: &Output) -> Result<bool> {
        let enabled = !output.enabled;
        self.repository
            .set_output_enabled(partition, output.id, enabled)
            .await?;
        Ok(enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Responde salidas fijas por partición y registra lo pedido.
    #[derive(Default)]
    struct RecordingOutputs {
        partitions: Vec<Partition>,
        requests: Vec<String>,
    }

    impl OutputRepository for RecordingOutputs {
        async fn partitions(&mut self) -> Result<Vec<PartitionName>> {
            Ok(self
                .partitions
                .iter()
                .map(|partition| partition.name.clone())
                .collect())
        }

        async fn outputs(&mut self, partition: &PartitionName) -> Result<Vec<Output>> {
            Ok(self
                .partitions
                .iter()
                .find(|candidate| candidate.name == *partition)
                .map(|candidate| candidate.outputs.clone())
                .unwrap_or_default())
        }

        async fn create_partition(&mut self, name: &PartitionName) -> Result<()> {
            self.requests.push(format!("create {name}"));
            Ok(())
        }

        async fn delete_partition(&mut self, name: &PartitionName) -> Result<()> {
            self.requests.push(format!("delete {name}"));
            Ok(())
        }

        async fn move_output(&mut self, output: &str, to: &PartitionName) -> Result<()> {
            self.requests.push(format!("move {output} -> {to}"));
            Ok(())
        }

        async fn set_output_enabled(
            &mut self,
            partition: &PartitionName,
            output_id: u32,
            enabled: bool,
        ) -> Result<()> {
            self.requests
                .push(format!("enable {partition}/{output_id} {enabled}"));
            Ok(())
        }
    }

    fn output(id: u32, name: &str, enabled: bool) -> Output {
        Output {
            id,
            name: name.into(),
            plugin: "alsa".into(),
            enabled,
        }
    }

    fn partition(name: &str, outputs: Vec<Output>) -> Partition {
        Partition {
            name: PartitionName::new(name).unwrap(),
            outputs,
        }
    }

    #[tokio::test]
    async fn overview_lists_each_partition_with_its_outputs() {
        let partitions = vec![
            partition("default", vec![output(0, "pulse", true)]),
            partition("cocina", vec![output(1, "parlante", false)]),
        ];
        let mut service = OutputService::new(RecordingOutputs {
            partitions: partitions.clone(),
            ..RecordingOutputs::default()
        });
        assert_eq!(service.overview().await.unwrap(), partitions);
    }

    #[tokio::test]
    async fn toggle_decides_from_the_known_state() {
        let mut service = OutputService::new(RecordingOutputs::default());
        let default = PartitionName::default_partition();
        assert!(
            service
                .toggle(&default, &output(2, "a", false))
                .await
                .unwrap()
        );
        assert!(
            !service
                .toggle(&default, &output(3, "b", true))
                .await
                .unwrap()
        );
        assert_eq!(
            service.repository.requests,
            ["enable default/2 true", "enable default/3 false"]
        );
    }

    #[tokio::test]
    async fn delete_returns_the_outputs_to_default_first() {
        let mut service = OutputService::new(RecordingOutputs::default());
        let cocina = partition("cocina", vec![output(0, "a", true), output(1, "b", false)]);
        service.delete(&cocina).await.unwrap();
        assert_eq!(
            service.repository.requests,
            ["move a -> default", "move b -> default", "delete cocina"]
        );
    }
}
