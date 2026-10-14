use std::{fmt, io};

/// Error devuelto por MPD en una línea `ACK [code@index] {command} message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ack {
    pub code: u32,
    pub command: String,
    pub message: String,
}

impl fmt::Display for Ack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.command.is_empty() {
            write!(formatter, "{}", self.message)
        } else {
            write!(formatter, "{}: {}", self.command, self.message)
        }
    }
}

/// Un error de E/S reducido a lo que se usa: su tipo y el texto del sistema. A
/// diferencia de `io::Error`, se puede clonar y comparar, así el error viaja tal cual
/// hasta la TUI, que elige las palabras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoFailure {
    kind: io::ErrorKind,
    message: String,
}

impl IoFailure {
    pub fn kind(&self) -> io::ErrorKind {
        self.kind
    }
}

impl fmt::Display for IoFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// Por qué falló un pedido a un servicio web.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpFailure {
    #[error("no respondió a tiempo")]
    Timeout,
    #[error("no se pudo conectar")]
    Connect,
    #[error("respuesta inválida")]
    InvalidResponse,
    /// Respondió con un código que no es de éxito.
    #[error("respondió {0}")]
    Status(u16),
    #[error("{0}")]
    Other(String),
}

/// El `Display` es el texto del log; lo que ve el usuario lo arma la TUI con el
/// catálogo del idioma (`i18n`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("error de E/S: {0}")]
    Io(IoFailure),
    #[error("MPD: {0}")]
    Ack(Ack),
    #[error("respuesta inválida de MPD: {0}")]
    Protocol(String),
    #[error("la conexión con MPD se cerró")]
    Closed,
    /// MPD no aceptó la conexión o no respondió dentro de `MPD_TIMEOUT`.
    #[error("MPD no respondió a tiempo")]
    Timeout,
    /// Un servicio web (LRCLIB) no respondió o respondió algo inesperado.
    #[error("{service}: {failure}")]
    Http {
        service: &'static str,
        failure: HttpFailure,
    },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(IoFailure {
            kind: error.kind(),
            message: error.to_string(),
        })
    }
}

impl Error {
    /// El error indica que la conexión ya no sirve y hay que abrir otra.
    pub fn is_disconnection(&self) -> bool {
        matches!(self, Error::Io(_) | Error::Closed)
    }

    /// La conexión quedó en un estado desconocido: no se vuelve a usar. A diferencia de
    /// una desconexión, un timeout no se reintenta (el comando pudo haberse ejecutado).
    pub fn leaves_connection_unusable(&self) -> bool {
        self.is_disconnection() || matches!(self, Error::Timeout)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
