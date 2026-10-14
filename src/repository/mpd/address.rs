use std::fmt;
use std::path::{Path, PathBuf};

pub const DEFAULT_HOST: &str = "localhost";
pub const DEFAULT_PORT: u16 = 6600;
const UNIX_SOCKET_PREFIX: char = '/';
/// Como en libmpdclient: `@nombre` es un socket abstracto de Linux.
const ABSTRACT_SOCKET_PREFIX: char = '@';
/// Socket de un MPD de usuario, dentro de `$XDG_RUNTIME_DIR`.
const RUNTIME_SOCKET: &str = "mpd/socket";
/// Socket de un MPD de sistema.
const SYSTEM_SOCKET: &str = "/run/mpd/socket";
const CANDIDATE_SEPARATOR: &str = ", ";

/// Dónde escucha MPD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Address {
    Tcp {
        host: String,
        port: u16,
    },
    Unix(PathBuf),
    /// Socket abstracto de Linux: el nombre, sin el `@`.
    Abstract(String),
}

impl Address {
    /// Igual que `mpc`: un host que empieza con `/` es un socket unix y uno que empieza
    /// con `@`, un socket abstracto.
    pub fn new(host: &str, port: u16) -> Self {
        if host.starts_with(UNIX_SOCKET_PREFIX) {
            Address::Unix(PathBuf::from(host))
        } else if let Some(name) = host.strip_prefix(ABSTRACT_SOCKET_PREFIX) {
            Address::Abstract(name.to_owned())
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
            Address::Abstract(name) => write!(formatter, "{ABSTRACT_SOCKET_PREFIX}{name}"),
        }
    }
}

/// Direcciones a probar en orden; nunca está vacía.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressCandidates(Vec<Address>);

impl AddressCandidates {
    /// Una lista no vacía por construcción: la primera y las que siguen.
    pub fn in_order(first: Address, rest: impl IntoIterator<Item = Address>) -> Self {
        Self(std::iter::once(first).chain(rest).collect())
    }

    /// Un host configurado es la única dirección.
    pub fn configured(address: Address) -> Self {
        Self::in_order(address, [])
    }

    /// El orden del manual de clientes de MPD: `$XDG_RUNTIME_DIR/mpd/socket` (si hay
    /// directorio), `/run/mpd/socket` y `localhost:port`.
    pub fn defaults(runtime_directory: Option<&Path>, port: u16) -> Self {
        let system_socket = Address::Unix(PathBuf::from(SYSTEM_SOCKET));
        let localhost = Address::new(DEFAULT_HOST, port);
        match runtime_directory.filter(|directory| !directory.as_os_str().is_empty()) {
            Some(directory) => {
                let runtime_socket = Address::Unix(directory.join(RUNTIME_SOCKET));
                Self::in_order(runtime_socket, [system_socket, localhost])
            }
            None => Self::in_order(system_socket, [localhost]),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.0.iter()
    }
}

impl fmt::Display for AddressCandidates {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addresses: Vec<String> = self.0.iter().map(Address::to_string).collect();
        formatter.write_str(&addresses.join(CANDIDATE_SEPARATOR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_are_tcp_unix_or_abstract_sockets() {
        assert_eq!(
            Address::new("servidor", 6601),
            Address::Tcp {
                host: "servidor".into(),
                port: 6601
            }
        );
        assert_eq!(
            Address::new("/run/mpd/socket", 6600),
            Address::Unix("/run/mpd/socket".into())
        );
        let abstract_socket = Address::new("@mpd", 6600);
        assert_eq!(abstract_socket, Address::Abstract("mpd".into()));
        assert_eq!(abstract_socket.to_string(), "@mpd");
    }

    #[test]
    fn default_candidates_follow_the_client_manual() {
        let with_runtime = AddressCandidates::defaults(Some(Path::new("/run/user/1000")), 6602);
        assert_eq!(
            with_runtime.to_string(),
            "/run/user/1000/mpd/socket, /run/mpd/socket, localhost:6602"
        );
        let without_runtime = AddressCandidates::defaults(None, 6600);
        assert_eq!(
            without_runtime.to_string(),
            "/run/mpd/socket, localhost:6600"
        );
        let empty_runtime = AddressCandidates::defaults(Some(Path::new("")), 6600);
        assert_eq!(empty_runtime, without_runtime);
    }

    #[test]
    fn a_configured_address_is_the_only_candidate() {
        let candidates = AddressCandidates::configured(Address::new("otro", 6600));
        assert_eq!(candidates.iter().count(), 1);
        assert_eq!(candidates.to_string(), "otro:6600");
    }
}
