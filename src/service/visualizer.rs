//! Tarea del visualizador: lee PCM de la fuente, analiza a `VISUALIZER_FPS` la forma
//! que pide la TUI y publica solo el último cuadro (`watch`), así no se acumula nada si
//! la TUI va más lenta. Corre solo mientras se la necesita: al soltar
//! `RunningVisualizer` se cancela.

use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};

use super::backoff::Backoff;
use super::visual::Analysis;
use crate::domain::{PcmFormat, VisualFrame, VisualRequest};
use crate::repository::{Error, PcmSource};

pub const VISUALIZER_FPS: u32 = 20;
const READ_BUFFER_BYTES: usize = 16 * 1024;

/// Lo que publica el visualizador.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualState {
    Frame(VisualFrame),
    /// No se puede abrir la fuente; la TUI explica el motivo.
    Unavailable(Error),
}

/// Antes del primer cuadro: nada que dibujar.
impl Default for VisualState {
    fn default() -> Self {
        Self::Frame(VisualFrame::default())
    }
}

pub struct Visualizer<S> {
    source: S,
    format: PcmFormat,
    backoff: Backoff,
    requests: watch::Receiver<VisualRequest>,
    states: watch::Sender<VisualState>,
}

impl<S: PcmSource + Send> Visualizer<S> {
    pub fn new(
        source: S,
        format: PcmFormat,
        backoff: Backoff,
        requests: watch::Receiver<VisualRequest>,
        states: watch::Sender<VisualState>,
    ) -> Self {
        Self {
            source,
            format,
            backoff,
            requests,
            states,
        }
    }

    /// Corre hasta que nadie escuche. Tras un fin de archivo o un error, vuelve a
    /// abrir la fuente con espera creciente, sin dejar de publicar cuadros.
    pub async fn run(mut self) {
        let mut analysis = Analysis::new(self.format, Instant::now());
        let mut buffer = vec![0; READ_BUFFER_BYTES];
        let mut frames = tokio::time::interval(Duration::from_secs(1) / VISUALIZER_FPS);
        frames.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut retry_at: Option<Instant> = None;
        let mut unavailable = false;
        loop {
            tokio::select! {
                read = self.source.read(&mut buffer), if retry_at.is_none() => match read {
                    Ok(0) => retry_at = Some(Instant::now() + self.backoff.next_delay()),
                    Ok(count) => {
                        self.backoff.reset();
                        unavailable = false;
                        analysis.receive(&buffer[..count], Instant::now());
                    }
                    Err(error) => {
                        unavailable = true;
                        retry_at = Some(Instant::now() + self.backoff.next_delay());
                        if !self.publish(VisualState::Unavailable(error)) {
                            return;
                        }
                    }
                },
                () = sleep_until(retry_at) => retry_at = None,
                now = frames.tick() => {
                    let frame = analysis.frame(*self.requests.borrow(), now);
                    if !unavailable && !self.publish(VisualState::Frame(frame)) {
                        return;
                    }
                }
            }
        }
    }

    /// `false` si ya nadie escucha.
    fn publish(&self, state: VisualState) -> bool {
        self.states.send(state).is_ok()
    }
}

/// Espera hasta `deadline`; sin plazo, para siempre.
async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

/// Abre una fuente nueva cada vez que se lanza el visualizador.
type SourceOpener<S> = Box<dyn Fn() -> S + Send + Sync>;

/// Lanza el visualizador cuando hace falta (al entrar a la vista Visual).
pub struct VisualizerLauncher<S> {
    open_source: SourceOpener<S>,
    /// Qué se lee, para mostrarlo (`/tmp/mpd.fifo`).
    source_label: String,
    format: PcmFormat,
    backoff: Backoff,
}

impl<S: PcmSource + Send + 'static> VisualizerLauncher<S> {
    pub fn new(
        open_source: impl Fn() -> S + Send + Sync + 'static,
        source_label: String,
        format: PcmFormat,
        backoff: Backoff,
    ) -> Self {
        Self {
            open_source: Box::new(open_source),
            source_label,
            format,
            backoff,
        }
    }

    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    pub fn format(&self) -> PcmFormat {
        self.format
    }

    pub fn launch(&self, request: VisualRequest) -> RunningVisualizer {
        let (requests_sender, requests_receiver) = watch::channel(request);
        let (states_sender, states_receiver) = watch::channel(VisualState::default());
        let visualizer = Visualizer::new(
            (self.open_source)(),
            self.format,
            self.backoff,
            requests_receiver,
            states_sender,
        );
        RunningVisualizer {
            task: tokio::spawn(visualizer.run()),
            requests: requests_sender,
            states: states_receiver,
        }
    }
}

/// El visualizador en marcha. Al soltarlo, la tarea se cancela y deja de leer.
pub struct RunningVisualizer {
    task: JoinHandle<()>,
    requests: watch::Sender<VisualRequest>,
    states: watch::Receiver<VisualState>,
}

impl RunningVisualizer {
    pub fn request(&self) -> VisualRequest {
        *self.requests.borrow()
    }

    /// Otra forma u otro tamaño: el próximo cuadro ya sale con lo nuevo.
    pub fn set_request(&self, request: VisualRequest) {
        self.requests.send_replace(request);
    }

    /// Espera el próximo estado publicado; `None` si la tarea terminó.
    pub async fn next_state(&mut self) -> Option<VisualState> {
        self.states.changed().await.ok()?;
        Some(self.states.borrow_and_update().clone())
    }
}

impl Drop for RunningVisualizer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io;

    use super::*;
    use crate::domain::{VisualContent, VisualizerStyle};
    use crate::repository::Result;
    use crate::service::visual::sine_pcm;

    /// Fuente en memoria: entrega cada lectura del guion y después espera para siempre.
    struct ScriptedSource {
        reads: VecDeque<Result<Vec<u8>>>,
    }

    impl PcmSource for ScriptedSource {
        async fn read(&mut self, destination: &mut [u8]) -> Result<usize> {
            match self.reads.pop_front() {
                Some(Ok(bytes)) => {
                    destination[..bytes.len()].copy_from_slice(&bytes);
                    Ok(bytes.len())
                }
                Some(Err(error)) => Err(error),
                None => std::future::pending().await,
            }
        }
    }

    const BANDS: usize = 16;
    const BARS: VisualRequest = VisualRequest {
        style: VisualizerStyle::Bars,
        columns: BANDS,
        rows: 0,
    };

    fn launcher(reads: Vec<Result<Vec<u8>>>) -> VisualizerLauncher<ScriptedSource> {
        let reads = std::sync::Mutex::new(Some(VecDeque::from(reads)));
        VisualizerLauncher::new(
            move || ScriptedSource {
                reads: reads.lock().unwrap().take().unwrap_or_default(),
            },
            "/tmp/prueba.fifo".into(),
            PcmFormat::parse("44100:16:2").unwrap(),
            Backoff::new(Duration::from_millis(50), Duration::from_secs(1)),
        )
    }

    /// Cuadros por segundo de sobra para que llegue lo esperado.
    const FRAMES_TO_WAIT: u32 = VISUALIZER_FPS * 5;

    /// El primer cuadro que cumple `wanted`; el primer tick puede llegar antes que los datos.
    async fn frame_where(
        running: &mut RunningVisualizer,
        wanted: impl Fn(&VisualFrame) -> bool,
    ) -> VisualFrame {
        for _ in 0..FRAMES_TO_WAIT {
            if let Some(VisualState::Frame(frame)) = running.next_state().await
                && wanted(&frame)
            {
                return frame;
            }
        }
        panic!("no llegó el cuadro esperado");
    }

    fn band_levels(frame: &VisualFrame) -> &[f32] {
        match &frame.content {
            VisualContent::Bars(bands) => &bands.levels,
            _ => &[],
        }
    }

    #[tokio::test(start_paused = true)]
    async fn bars_follow_the_signal_and_fall_without_it() {
        let chunk = sine_pcm(1_000.0, 0.5, READ_BUFFER_BYTES / 4);
        let mut running = launcher(vec![Ok(chunk)]).launch(BARS);
        let frame = frame_where(&mut running, |frame| frame.signal).await;
        assert_eq!(band_levels(&frame).len(), BANDS);
        assert!(band_levels(&frame).iter().any(|level| *level > 0.5));
        let frame = frame_where(&mut running, |frame| !frame.signal).await;
        assert!(!frame.content.is_flat());
        frame_where(&mut running, |frame| frame.content.is_flat()).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_missing_source_is_explained_and_retried() {
        let missing = Error::from(io::Error::from(io::ErrorKind::NotFound));
        let chunk = sine_pcm(1_000.0, 0.5, 1_024);
        let mut running = launcher(vec![Err(missing.clone()), Ok(chunk)]).launch(BARS);
        assert_eq!(
            running.next_state().await,
            Some(VisualState::Unavailable(missing))
        );
        frame_where(&mut running, |frame| frame.signal).await;
    }

    #[tokio::test(start_paused = true)]
    async fn the_request_can_change_while_running() {
        let mut running = launcher(Vec::new()).launch(BARS);
        let wider = VisualRequest {
            columns: BANDS * 2,
            ..BARS
        };
        running.set_request(wider);
        assert_eq!(running.request(), wider);
        frame_where(&mut running, |frame| band_levels(frame).len() == BANDS * 2).await;
        running.set_request(VisualRequest {
            style: VisualizerStyle::Meters,
            ..BARS
        });
        frame_where(&mut running, |frame| {
            frame.content.style() == VisualizerStyle::Meters
        })
        .await;
    }
}
