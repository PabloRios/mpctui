//! Transporte: envía un comando y devuelve los pares de la respuesta.

use std::future::Future;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream as StdUnixStream};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UnixStream};

use super::address::{Address, AddressCandidates};
use super::protocol::{
    Command, Pair, ResponseLine, check_greeting, continue_value, parse_response_line,
};
use super::reconnect::Connector;
use super::vocabulary::Verb;
use crate::domain::Password;
use crate::repository::error::{Error, Result};

const LINE_END: u8 = b'\n';
/// Lo que recomienda el manual de clientes de MPD para `MPD_TIMEOUT`.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Contrato mínimo que necesita `MpdClient`; permite inyectar transportes falsos.
pub trait Transport {
    /// Envía el comando y lee hasta `OK`; un `ACK` vuelve como `Error::Ack`.
    fn execute(&mut self, command: &Command) -> impl Future<Output = Result<Vec<Pair>>> + Send;
}

/// Cómo se abre cada conexión: la contraseña, si hay, y cuánto se espera a MPD al
/// conectar y en cada respuesta.
#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub password: Option<Password>,
    pub timeout: Duration,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            password: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

type BoxedReader = Box<dyn AsyncRead + Send + Unpin>;
type BoxedWriter = Box<dyn AsyncWrite + Send + Unpin>;

/// Conexión real por TCP, socket unix o socket abstracto.
pub struct Connection {
    reader: BufReader<BoxedReader>,
    writer: BoxedWriter,
    timeout: Duration,
}

impl Connection {
    /// Abre la conexión, valida el saludo y se autentica si hay contraseña, todo dentro
    /// del timeout.
    pub async fn connect(address: &Address, settings: &ConnectionSettings) -> Result<Self> {
        within(settings.timeout, async {
            let (reader, writer) = open_stream(address).await?;
            Self::start(reader, writer, settings).await
        })
        .await
    }

    /// Valida el saludo y se autentica sobre un flujo ya abierto.
    async fn start(
        reader: BoxedReader,
        writer: BoxedWriter,
        settings: &ConnectionSettings,
    ) -> Result<Self> {
        let mut connection = Self {
            reader: BufReader::new(reader),
            writer,
            timeout: settings.timeout,
        };
        check_greeting(&connection.read_line().await?)?;
        if let Some(password) = &settings.password {
            connection
                .exchange(&Command::new(Verb::Password).arg(password.as_str()))
                .await?;
        }
        Ok(connection)
    }

    async fn exchange(&mut self, command: &Command) -> Result<Vec<Pair>> {
        self.send(command).await?;
        self.read_response().await
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
                ResponseLine::Continuation(line) => continue_value(&mut pairs, &line)?,
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
    /// `idle` espera sin plazo; el resto de los comandos, a lo sumo el timeout.
    async fn execute(&mut self, command: &Command) -> Result<Vec<Pair>> {
        if command.is_idle() {
            self.exchange(command).await
        } else {
            let timeout = self.timeout;
            within(timeout, self.exchange(command)).await
        }
    }
}

/// Corre la operación con un plazo; si se vence, es `Error::Timeout`.
async fn within<T>(timeout: Duration, operation: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(timeout, operation)
        .await
        .unwrap_or(Err(Error::Timeout))
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
        Address::Abstract(name) => {
            let (reader, writer) = connect_abstract(name).await?.into_split();
            Ok((Box::new(reader), Box::new(writer)))
        }
    }
}

/// tokio no conecta a sockets abstractos: se conecta con la biblioteca estándar (en
/// una tarea bloqueante) y se pasa el socket a tokio.
async fn connect_abstract(name: &str) -> Result<UnixStream> {
    let socket_address = SocketAddr::from_abstract_name(name)?;
    let stream = tokio::task::spawn_blocking(move || StdUnixStream::connect_addr(&socket_address))
        .await
        .map_err(|error| Error::from(std::io::Error::other(error)))??;
    stream.set_nonblocking(true)?;
    Ok(UnixStream::from_std(stream)?)
}

/// La primera dirección que acepta la conexión. Solo los errores de conexión pasan a la
/// siguiente: un `ACK` (una contraseña inválida) quiere decir que MPD está ahí. Si
/// ninguna responde, devuelve el error de la última.
pub async fn first_reachable(
    candidates: &AddressCandidates,
    settings: &ConnectionSettings,
) -> Result<Address> {
    let mut last_error = Error::Closed;
    for address in candidates.iter() {
        match Connection::connect(address, settings).await {
            Ok(_) => return Ok(address.clone()),
            Err(error) if error.leaves_connection_unusable() => last_error = error,
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

/// Sabe abrir conexiones reales a un MPD concreto.
pub struct MpdConnector {
    address: Address,
    settings: ConnectionSettings,
}

impl MpdConnector {
    pub fn new(address: Address, settings: ConnectionSettings) -> Self {
        Self { address, settings }
    }
}

impl Connector for MpdConnector {
    type Transport = Connection;

    async fn connect(&self) -> Result<Connection> {
        Connection::connect(&self.address, &self.settings).await
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{DuplexStream, duplex};

    use super::*;

    const BUFFER_SIZE: usize = 1024;
    const TIMEOUT: Duration = Duration::from_secs(5);
    const GREETING: &[u8] = b"OK MPD 0.23.5\n";

    fn settings() -> ConnectionSettings {
        ConnectionSettings {
            password: None,
            timeout: TIMEOUT,
        }
    }

    /// Una conexión sobre un flujo en memoria; el otro extremo hace de servidor.
    async fn start_in_memory(server_writes: &[u8]) -> (Result<Connection>, DuplexStream) {
        let (client, mut server) = duplex(BUFFER_SIZE);
        server.write_all(server_writes).await.unwrap();
        let (reader, writer) = tokio::io::split(client);
        let connection = within(
            TIMEOUT,
            Connection::start(Box::new(reader), Box::new(writer), &settings()),
        )
        .await;
        (connection, server)
    }

    #[tokio::test(start_paused = true)]
    async fn a_server_without_greeting_times_out() {
        let (connection, _server) = start_in_memory(b"").await;
        assert!(matches!(connection, Err(Error::Timeout)));
    }

    #[tokio::test(start_paused = true)]
    async fn a_missing_response_times_out() {
        let (connection, _server) = start_in_memory(GREETING).await;
        let result = connection
            .unwrap()
            .execute(&Command::new(Verb::Status))
            .await;
        assert!(matches!(result, Err(Error::Timeout)));
    }

    #[tokio::test(start_paused = true)]
    async fn idle_waits_longer_than_the_timeout() {
        let (connection, mut server) = start_in_memory(GREETING).await;
        let responder = tokio::spawn(async move {
            tokio::time::sleep(TIMEOUT * 3).await;
            server.write_all(b"changed: player\nOK\n").await.unwrap();
            server
        });
        let pairs = connection
            .unwrap()
            .execute(&Command::new(Verb::Idle))
            .await
            .unwrap();
        assert_eq!(pairs, [("changed".to_owned(), "player".to_owned())]);
        drop(responder.await.unwrap());
    }

    #[tokio::test]
    async fn an_unreachable_abstract_socket_is_an_io_error() {
        let address = Address::Abstract("mpctui-prueba-sin-servidor".into());
        let result = Connection::connect(&address, &settings()).await;
        assert!(matches!(result, Err(Error::Io(_))));
    }
}
