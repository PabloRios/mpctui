//! Transporte: envía un comando y devuelve los pares de la respuesta.

use std::future::Future;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UnixStream};

use super::address::Address;
use super::protocol::{Command, Pair, ResponseLine, check_greeting, parse_response_line};
use super::reconnect::Connector;
use super::vocabulary::Verb;
use crate::domain::Password;
use crate::repository::error::{Error, Result};

const LINE_END: u8 = b'\n';

/// Contrato mínimo que necesita `MpdClient`; permite inyectar transportes falsos.
pub trait Transport {
    /// Envía el comando y lee hasta `OK`; un `ACK` vuelve como `Error::Ack`.
    fn execute(&mut self, command: &Command) -> impl Future<Output = Result<Vec<Pair>>> + Send;
}

type BoxedReader = Box<dyn AsyncRead + Send + Unpin>;
type BoxedWriter = Box<dyn AsyncWrite + Send + Unpin>;

/// Conexión real por TCP o socket unix.
pub struct Connection {
    reader: BufReader<BoxedReader>,
    writer: BoxedWriter,
}

impl Connection {
    /// Abre la conexión, valida el saludo y se autentica si hay contraseña.
    pub async fn connect(address: &Address, password: Option<&Password>) -> Result<Self> {
        let (reader, writer) = open_stream(address).await?;
        let mut connection = Self {
            reader: BufReader::new(reader),
            writer,
        };
        check_greeting(&connection.read_line().await?)?;
        if let Some(password) = password {
            connection
                .execute(&Command::new(Verb::Password).arg(password.as_str()))
                .await?;
        }
        Ok(connection)
    }

    async fn send(&mut self, command: &Command) -> Result<()> {
        self.writer.write_all(command.as_str().as_bytes()).await?;
        self.writer.write_all(&[LINE_END]).await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn read_response(&mut self) -> Result<Vec<Pair>> {
        let mut pairs = Vec::new();
        loop {
            match parse_response_line(&self.read_line().await?)? {
                ResponseLine::Ok => return Ok(pairs),
                ResponseLine::Ack(ack) => return Err(Error::Ack(ack)),
                ResponseLine::Pair(pair) => pairs.push(pair),
            }
        }
    }

    async fn read_line(&mut self) -> Result<String> {
        let mut line_bytes = Vec::new();
        if self.reader.read_until(LINE_END, &mut line_bytes).await? == 0 {
            return Err(Error::Closed);
        }
        if line_bytes.last() == Some(&LINE_END) {
            line_bytes.pop();
        }
        Ok(String::from_utf8_lossy(&line_bytes).into_owned())
    }
}

impl Transport for Connection {
    async fn execute(&mut self, command: &Command) -> Result<Vec<Pair>> {
        self.send(command).await?;
        self.read_response().await
    }
}

async fn open_stream(address: &Address) -> Result<(BoxedReader, BoxedWriter)> {
    match address {
        Address::Tcp { host, port } => {
            let stream = TcpStream::connect((host.as_str(), *port)).await?;
            stream.set_nodelay(true)?;
            let (reader, writer) = stream.into_split();
            Ok((Box::new(reader), Box::new(writer)))
        }
        Address::Unix(path) => {
            let (reader, writer) = UnixStream::connect(path).await?.into_split();
            Ok((Box::new(reader), Box::new(writer)))
        }
    }
}

/// Sabe abrir conexiones reales a un MPD concreto.
pub struct MpdConnector {
    address: Address,
    password: Option<Password>,
}

impl MpdConnector {
    pub fn new(address: Address, password: Option<Password>) -> Self {
        Self { address, password }
    }
}

impl Connector for MpdConnector {
    type Transport = Connection;

    async fn connect(&self) -> Result<Connection> {
        Connection::connect(&self.address, self.password.as_ref()).await
    }
}
