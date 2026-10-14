//! Vigila los cambios del servidor (`idle`) y los publica como eventos. Si se pierde la
//! conexión, reintenta con espera creciente y avisa cuándo será el próximo intento.

use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use super::backoff::Backoff;
use crate::domain::Subsystem;
use crate::repository::{ChangesRepository, Error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    Changed(Vec<Subsystem>),
    /// No se pudo hablar con el servidor: el intento número `attempt` será en `retry_at`,
    /// después de esperar `delay`.
    Disconnected {
        attempt: u32,
        retry_at: Instant,
        delay: Duration,
        reason: Error,
    },
    /// Se está intentando reconectar.
    Connecting {
        attempt: u32,
    },
    /// El servidor volvió a responder.
    Connected,
}

pub struct ChangeWatcher<R> {
    repository: R,
    events: UnboundedSender<ServerEvent>,
    backoff: Backoff,
}

impl<R: ChangesRepository + Send> ChangeWatcher<R> {
    pub fn new(repository: R, events: UnboundedSender<ServerEvent>, backoff: Backoff) -> Self {
        Self {
            repository,
            events,
            backoff,
        }
    }

    /// Corre hasta que nadie escuche los eventos.
    pub async fn run(mut self) {
        loop {
            let listening = match self.repository.wait_for_changes().await {
                Ok(changed) => self.publish(ServerEvent::Changed(changed)),
                Err(error) => self.reconnect(error).await,
            };
            if !listening {
                return;
            }
        }
    }

    /// Reintenta con espera creciente hasta que el servidor responda. Devuelve `false`
    /// si nadie escucha los eventos.
    async fn reconnect(&mut self, first_reason: Error) -> bool {
        let mut reason = first_reason;
        let mut attempt: u32 = 1;
        loop {
            let delay = self.backoff.next_delay();
            let retry_at = Instant::now() + delay;
            let waiting = ServerEvent::Disconnected {
                attempt,
                retry_at,
                delay,
                reason,
            };
            if !self.publish(waiting) {
                return false;
            }
            tokio::time::sleep_until(retry_at).await;
            if !self.publish(ServerEvent::Connecting { attempt }) {
                return false;
            }
            match self.repository.ping().await {
                Ok(()) => {
                    self.backoff.reset();
                    return self.publish(ServerEvent::Connected);
                }
                Err(error) => reason = error,
            }
            attempt = attempt.saturating_add(1);
        }
    }

    /// `false` si ya nadie escucha.
    fn publish(&self, event: ServerEvent) -> bool {
        self.events.send(event).is_ok()
    }
}

/// Abre el repositorio de un vigilante nuevo.
type RepositoryOpener<R> = Box<dyn Fn() -> R + Send + Sync>;

/// Lanza vigilantes. `idle` bloquea la conexión en una partición: para vigilar otra se
/// suelta el vigilante y se lanza uno nuevo, con una conexión nueva.
pub struct WatcherLauncher<R> {
    open_repository: RepositoryOpener<R>,
    events: UnboundedSender<ServerEvent>,
    backoff: Backoff,
}

impl<R: ChangesRepository + Send + 'static> WatcherLauncher<R> {
    /// `open_repository` no debería conectar: si el servidor no responde, el vigilante
    /// ya reintenta con su espera creciente.
    pub fn new(
        open_repository: impl Fn() -> R + Send + Sync + 'static,
        events: UnboundedSender<ServerEvent>,
        backoff: Backoff,
    ) -> Self {
        Self {
            open_repository: Box::new(open_repository),
            events,
            backoff,
        }
    }

    pub fn launch(&self) -> RunningWatcher {
        let watcher =
            ChangeWatcher::new((self.open_repository)(), self.events.clone(), self.backoff);
        RunningWatcher {
            task: tokio::spawn(watcher.run()),
        }
    }
}

/// El vigilante en marcha. Al soltarlo, la tarea se cancela y su conexión se cierra.
pub struct RunningWatcher {
    task: JoinHandle<()>,
}

impl Drop for RunningWatcher {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use tokio::sync::mpsc;

    use super::*;
    use crate::repository::Result;

    /// Devuelve las respuestas del guion y después espera para siempre.
    #[derive(Default)]
    struct ScriptedChanges {
        changes: VecDeque<Result<Vec<Subsystem>>>,
        pings: VecDeque<Result<()>>,
    }

    impl ChangesRepository for ScriptedChanges {
        async fn wait_for_changes(&mut self) -> Result<Vec<Subsystem>> {
            match self.changes.pop_front() {
                Some(response) => response,
                None => std::future::pending().await,
            }
        }

        async fn ping(&mut self) -> Result<()> {
            self.pings.pop_front().unwrap_or(Ok(()))
        }
    }

    const FIRST_RETRY: Duration = Duration::from_secs(1);

    fn start(script: ScriptedChanges) -> mpsc::UnboundedReceiver<ServerEvent> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let backoff = Backoff::new(FIRST_RETRY, Duration::from_secs(30));
        tokio::spawn(ChangeWatcher::new(script, sender, backoff).run());
        receiver
    }

    #[tokio::test(start_paused = true)]
    async fn publishes_changes() {
        let mut events = start(ScriptedChanges {
            changes: VecDeque::from([Ok(vec![Subsystem::Player]), Ok(vec![Subsystem::Mixer])]),
            ..ScriptedChanges::default()
        });
        assert_eq!(
            events.recv().await,
            Some(ServerEvent::Changed(vec![Subsystem::Player]))
        );
        assert_eq!(
            events.recv().await,
            Some(ServerEvent::Changed(vec![Subsystem::Mixer]))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn announces_each_retry_until_the_server_answers() {
        let mut events = start(ScriptedChanges {
            changes: VecDeque::from([Err(Error::Closed), Ok(vec![Subsystem::Player])]),
            pings: VecDeque::from([Err(Error::Closed), Ok(())]),
        });
        let started = Instant::now();

        let Some(ServerEvent::Disconnected {
            attempt: 1,
            retry_at,
            delay,
            ..
        }) = events.recv().await
        else {
            panic!("se esperaba la primera desconexión");
        };
        assert_eq!(delay, FIRST_RETRY);
        assert_eq!(retry_at, started + FIRST_RETRY);
        assert_eq!(
            events.recv().await,
            Some(ServerEvent::Connecting { attempt: 1 })
        );
        assert_eq!(Instant::now(), started + FIRST_RETRY);

        let Some(ServerEvent::Disconnected {
            attempt: 2,
            retry_at,
            delay,
            ..
        }) = events.recv().await
        else {
            panic!("se esperaba el segundo intento");
        };
        assert_eq!(delay, FIRST_RETRY * 2);
        assert_eq!(retry_at, started + FIRST_RETRY * 3);
        assert_eq!(
            events.recv().await,
            Some(ServerEvent::Connecting { attempt: 2 })
        );
        assert_eq!(events.recv().await, Some(ServerEvent::Connected));
        assert_eq!(
            events.recv().await,
            Some(ServerEvent::Changed(vec![Subsystem::Player]))
        );
    }

    /// Espera para siempre; mientras vive, retiene una copia de `alive`.
    struct Idle {
        _alive: std::sync::Arc<()>,
    }

    impl ChangesRepository for Idle {
        async fn wait_for_changes(&mut self) -> Result<Vec<Subsystem>> {
            std::future::pending().await
        }

        async fn ping(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn dropping_a_running_watcher_closes_its_repository() {
        let alive = std::sync::Arc::new(());
        let (sender, _receiver) = mpsc::unbounded_channel();
        let backoff = Backoff::new(FIRST_RETRY, Duration::from_secs(30));
        let opened = alive.clone();
        let launcher = WatcherLauncher::new(
            move || Idle {
                _alive: opened.clone(),
            },
            sender,
            backoff,
        );
        let first = launcher.launch();
        let second = launcher.launch();
        tokio::task::yield_now().await;
        assert_eq!(std::sync::Arc::strong_count(&alive), 4);
        drop(first);
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert_eq!(std::sync::Arc::strong_count(&alive), 3);
        drop(second);
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert_eq!(std::sync::Arc::strong_count(&alive), 2);
    }
}
