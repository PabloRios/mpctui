//! Implementación de los repositorios con un cliente propio del protocolo de MPD.

mod address;
mod client;
mod parse;
mod partition;
mod protocol;
mod reconnect;
mod transport;
mod vocabulary;

pub use address::{Address, AddressCandidates, DEFAULT_HOST, DEFAULT_PORT};
pub use client::MpdClient;
pub use partition::{PartitionConnector, PartitionTransport, SharedPartition};
pub use protocol::{Command, Pair};
pub use reconnect::{Connector, ReconnectingTransport};
pub use transport::{
    Connection, ConnectionSettings, DEFAULT_TIMEOUT, MpdConnector, Transport, first_reachable,
};
