//! La partición que usa mpctui: todas las conexiones que la siguen entran a ella antes
//! de su próximo comando, tanto al cambiarla como al reconectar (una conexión nueva
//! arranca en `default`).

use std::sync::{Arc, PoisonError, RwLock};

use super::protocol::{Command, Pair};
use super::reconnect::Connector;
use super::transport::Transport;
use super::vocabulary::{ACK_NO_EXIST, Verb};
use crate::domain::PartitionName;
use crate::repository::error::{Error, Result};
use crate::repository::traits::PartitionSelection;

/// La partición elegida, compartida entre todas las conexiones que la siguen.
#[derive(Debug, Clone, Default)]
pub struct SharedPartition(Arc<RwLock<PartitionName>>);

impl SharedPartition {
    /// Vuelve a `default` si la elegida sigue siendo `gone`; si mientras tanto se eligió
    /// otra, la respeta.
    fn forget(&self, gone: &PartitionName) {
        let mut active = self.0.write().unwrap_or_else(PoisonError::into_inner);
        if *active == *gone {
            *active = PartitionName::default_partition();
        }
    }
}

impl PartitionSelection for SharedPartition {
    fn active(&self) -> PartitionName {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn select(&self, partition: PartitionName) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = partition;
    }
}

/// Abre conexiones que siguen la partición compartida.
pub struct PartitionConnector<C> {
    connector: C,
    partition: SharedPartition,
}

impl<C> PartitionConnector<C> {
    pub fn new(connector: C, partition: SharedPartition) -> Self {
        Self {
            connector,
            partition,
        }
    }
}

impl<C: Connector + Send + Sync> Connector for PartitionConnector<C> {
    type Transport = PartitionTransport<C::Transport>;

    async fn connect(&self) -> Result<Self::Transport> {
        Ok(PartitionTransport {
            transport: self.connector.connect().await?,
            partition: self.partition.clone(),
            current: PartitionName::default_partition(),
        })
    }
}

/// Una conexión que, antes de cada comando, entra a la partición elegida si no está en
/// ella.
pub struct PartitionTransport<T> {
    transport: T,
    partition: SharedPartition,
    /// Dónde está la conexión.
    current: PartitionName,
}

impl<T: Transport + Send> PartitionTransport<T> {
    /// Si la elegida ya no existe (MPD se reinició), se vuelve a `default` y se devuelve
    /// el error: el próximo comando ya va a `default`.
    async fn follow(&mut self) -> Result<()> {
        let wanted = self.partition.active();
        if wanted == self.current {
            return Ok(());
        }
        let switch = Command::new(Verb::Partition).arg(&wanted);
        match self.transport.execute(&switch).await {
            Ok(_) => {
                self.current = wanted;
                Ok(())
            }
            Err(Error::Ack(ack)) if ack.code == ACK_NO_EXIST => {
                self.partition.forget(&wanted);
                Err(Error::Ack(ack))
            }
            Err(error) => Err(error),
        }
    }
}

impl<T: Transport + Send> Transport for PartitionTransport<T> {
    async fn execute(&mut self, command: &Command) -> Result<Vec<Pair>> {
        self.follow().await?;
        self.transport.execute(command).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::repository::error::Ack;

    /// Registra los comandos de todas sus conexiones; `partition` a una partición de
    /// `missing` responde que no existe.
    #[derive(Default, Clone)]
    struct Recorder {
        sent: Arc<Mutex<Vec<String>>>,
        missing: Vec<&'static str>,
    }

    impl Recorder {
        fn sent(&self) -> Vec<String> {
            self.sent.lock().unwrap().clone()
        }
    }

    impl Transport for Recorder {
        async fn execute(&mut self, command: &Command) -> Result<Vec<Pair>> {
            self.sent.lock().unwrap().push(command.as_str().to_owned());
            let missing = self
                .missing
                .iter()
                .any(|name| command.as_str() == format!("partition \"{name}\""));
            if missing {
                return Err(Error::Ack(Ack {
                    code: ACK_NO_EXIST,
                    command: "partition".into(),
                    message: "partition does not exist".into(),
                }));
            }
            Ok(Vec::new())
        }
    }

    impl Connector for Recorder {
        type Transport = Recorder;

        async fn connect(&self) -> Result<Recorder> {
            Ok(self.clone())
        }
    }

    fn name(raw: &str) -> PartitionName {
        PartitionName::new(raw).unwrap()
    }

    #[tokio::test]
    async fn enters_the_chosen_partition_once_before_the_next_command() {
        let recorder = Recorder::default();
        let shared = SharedPartition::default();
        let connector = PartitionConnector::new(recorder.clone(), shared.clone());
        let mut transport = connector.connect().await.unwrap();
        let status = Command::new(Verb::Status);
        transport.execute(&status).await.unwrap();
        shared.select(name("cocina"));
        transport.execute(&status).await.unwrap();
        transport.execute(&status).await.unwrap();
        assert_eq!(
            recorder.sent(),
            ["status", "partition \"cocina\"", "status", "status"]
        );
    }

    #[tokio::test]
    async fn a_new_connection_enters_again() {
        let recorder = Recorder::default();
        let shared = SharedPartition::default();
        shared.select(name("cocina"));
        let connector = PartitionConnector::new(recorder.clone(), shared);
        for _ in 0..2 {
            let mut transport = connector.connect().await.unwrap();
            transport.execute(&Command::new(Verb::Ping)).await.unwrap();
        }
        assert_eq!(
            recorder.sent(),
            [
                "partition \"cocina\"",
                "ping",
                "partition \"cocina\"",
                "ping"
            ]
        );
    }

    #[tokio::test]
    async fn a_missing_partition_falls_back_to_default() {
        let recorder = Recorder {
            missing: vec!["cocina"],
            ..Recorder::default()
        };
        let shared = SharedPartition::default();
        shared.select(name("cocina"));
        let connector = PartitionConnector::new(recorder.clone(), shared.clone());
        let mut transport = connector.connect().await.unwrap();
        let status = Command::new(Verb::Status);
        assert!(matches!(
            transport.execute(&status).await,
            Err(Error::Ack(ack)) if ack.code == ACK_NO_EXIST
        ));
        assert!(shared.active().is_default());
        transport.execute(&status).await.unwrap();
        assert_eq!(recorder.sent(), ["partition \"cocina\"", "status"]);
    }

    #[test]
    fn forgetting_respects_a_newer_choice() {
        let shared = SharedPartition::default();
        shared.select(name("living"));
        shared.forget(&name("cocina"));
        assert_eq!(shared.active(), name("living"));
    }
}
