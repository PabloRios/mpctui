use std::fmt;
use std::path::PathBuf;

pub const DEFAULT_HOST: &str = "localhost";
pub const DEFAULT_PORT: u16 = 6600;
const UNIX_SOCKET_PREFIX: char = '/';

/// Dónde escucha MPD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    Tcp { host: String, port: u16 },
    Unix(PathBuf),
}

impl Address {
    /// Igual que `mpc`: un host que empieza con `/` es un socket unix.
    pub fn new(host: &str, port: u16) -> Self {
        if host.starts_with(UNIX_SOCKET_PREFIX) {
            Address::Unix(PathBuf::from(host))
        } else {
            Address::Tcp {
                host: host.to_owned(),
                port,
            }
        }
    }
}

impl Default for Address {
    fn default() -> Self {
        Address::new(DEFAULT_HOST, DEFAULT_PORT)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Tcp { host, port } => write!(formatter, "{host}:{port}"),
            Address::Unix(path) => write!(formatter, "{}", path.display()),
        }
    }
}
