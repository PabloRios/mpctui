//! Transporte que reabre la conexión cuando se corta (MPD cierra las conexiones
//! inactivas después de `connection_timeout`, 60 s por defecto).

use std::future::Future;

use super::protocol::{Command, Pair};
use super::transport::Transport;
use crate::repository::error::Result;

/// Fábrica de transportes; se inyecta para no acoplar la reconexión a una conexión concreta.
pub trait Connector {
    type Transport: Transport + Send;

    fn connect(&self) -> impl Future<Output = Result<Self::Transport>> + Send;
}

pub struct ReconnectingTransport<C: Connector> {
    connector: C,
    current: Option<C::Transport>,
}

impl<C: Connector + Send + Sync> ReconnectingTransport<C> {
    /// Conecta de inmediato para fallar rápido si el servidor no está disponible.
    pub async fn connect(connector: C) -> Result<Self> {
        let current = connector.connect().await?;
        Ok(Self {
            connector,
            current: Some(current),
        })
    }

    /// Conecta recién con el primer comando: para quien ya sabe reintentar (el vigilante
    /// de cambios relanzado al cambiar de partición).
    pub fn lazy(connector: C) -> Self {
        Self {
            connector,
            current: None,
        }
    }

    /// Ejecuta sobre la conexión actual (o una nueva) y la descarta si se cortó.
    async fn execute_once(&mut self, command: &Command) -> Result<Vec<Pair>> {
        let mut transport = match self.current.take() {
            Some(transport) => transport,
            None => self.connector.connect().await?,
        };
        let result = transport.execute(command).await;
        if !matches!(&result, Err(error) if error.is_disconnection()) {
            self.current = Some(transport);
        }
        result
    }
}

impl<C: Connector + Send + Sync> Transport for ReconnectingTransport<C> {
    /// Si la conexión estaba cortada, reintenta una sola vez con una conexión nueva.
    async fn execute(&mut self, command: &Command) -> Result<Vec<Pair>> {
        match self.execute_once(command).await {
            Err(error) if error.is_disconnection() => self.execute_once(command).await,
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::repository::error::{Ack, Error};
    use crate::repository::mpd::vocabulary::Verb;

    /// Transporte falso que responde siempre lo mismo.
    struct FixedTransport {
        fails_with_closed: bool,
    }

    impl Transport for FixedTransport {
        async fn execute(&mut self, _command: &Command) -> Result<Vec<Pair>> {
            if self.fails_with_closed {
                Err(Error::Closed)
            } else {
                Ok(Vec::new())
            }
        }
    }

    /// La primera conexión viene cortada; las siguientes funcionan.
    #[derive(Default)]
    struct FirstConnectionBroken {
        connections: AtomicUsize,
    }

    impl Connector for FirstConnectionBroken {
        type Transport = FixedTransport;

        async fn connect(&self) -> Result<FixedTransport> {
            let previous = self.connections.fetch_add(1, Ordering::SeqCst);
            Ok(FixedTransport {
                fails_with_closed: previous == 0,
            })
        }
    }

    struct AckTransport;

    impl Transport for AckTransport {
        async fn execute(&mut self, _command: &Command) -> Result<Vec<Pair>> {
            Err(Error::Ack(Ack {
                code: 2,
                command: "play".into(),
                message: "Bad song index".into(),
            }))
        }
    }

    #[derive(Default)]
    struct AckConnector {
        connections: AtomicUsize,
    }

    impl Connector for AckConnector {
        type Transport = AckTransport;

        async fn connect(&self) -> Result<AckTransport> {
            self.connections.fetch_add(1, Ordering::SeqCst);
            Ok(AckTransport)
        }
    }

    #[tokio::test]
    async fn reconnects_once_when_connection_was_closed() {
        let mut transport = ReconnectingTransport::connect(FirstConnectionBroken::default())
            .await
            .unwrap();
        transport
            .execute(&Command::new(Verb::Status))
            .await
            .unwrap();
        assert_eq!(transport.connector.connections.load(Ordering::SeqCst), 2);
        transport
            .execute(&Command::new(Verb::Status))
            .await
            .unwrap();
        assert_eq!(transport.connector.connections.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_lazy_transport_connects_with_the_first_command() {
        let mut transport = ReconnectingTransport::lazy(AckConnector::default());
        assert_eq!(transport.connector.connections.load(Ordering::SeqCst), 0);
        let result = transport.execute(&Command::new(Verb::Play)).await;
        assert!(matches!(result, Err(Error::Ack(_))));
        assert_eq!(transport.connector.connections.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn keeps_connection_on_server_errors() {
        let mut transport = ReconnectingTransport::connect(AckConnector::default())
            .await
            .unwrap();
        let result = transport.execute(&Command::new(Verb::Play)).await;
        assert!(matches!(result, Err(Error::Ack(_))));
        assert_eq!(transport.connector.connections.load(Ordering::SeqCst), 1);
    }
}
