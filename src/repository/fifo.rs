//! Lectura de la salida fifo de MPD. No es parte del protocolo de MPD: es un pipe con
//! PCM crudo que el servidor escribe mientras suena.

use std::path::PathBuf;

use tokio::io::AsyncReadExt;
use tokio::net::unix::pipe;

use super::error::Result;
use super::traits::PcmSource;

/// Abre el FIFO en la primera lectura (y de nuevo después de un fin de archivo), en
/// modo no bloqueante: abrir no espera a que MPD esté escribiendo.
pub struct FifoPcmSource {
    path: PathBuf,
    receiver: Option<pipe::Receiver>,
}

impl FifoPcmSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            receiver: None,
        }
    }
}

impl PcmSource for FifoPcmSource {
    async fn read(&mut self, destination: &mut [u8]) -> Result<usize> {
        let receiver = match &mut self.receiver {
            Some(receiver) => receiver,
            None => self
                .receiver
                .insert(pipe::OpenOptions::new().open_receiver(&self.path)?),
        };
        let count = receiver.read(destination).await?;
        if count == 0 {
            self.receiver = None;
        }
        Ok(count)
    }
}
