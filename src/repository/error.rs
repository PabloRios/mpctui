use std::fmt;

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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("MPD: {0}")]
    Ack(Ack),
    #[error("respuesta inválida de MPD: {0}")]
    Protocol(String),
    #[error("la conexión con MPD se cerró")]
    Closed,
}

impl Error {
    /// El error indica que la conexión ya no sirve y hay que abrir otra.
    pub fn is_disconnection(&self) -> bool {
        matches!(self, Error::Io(_) | Error::Closed)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
