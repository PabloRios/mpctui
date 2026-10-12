//! Tarea del visualizador: lee PCM de la fuente, analiza a `VISUALIZER_FPS` y publica
//! solo el último cuadro (`watch`), así no se acumula nada si la TUI va más lenta.
//! Corre solo mientras se la necesita: al soltar `RunningVisualizer` se cancela.

use std::io;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};

use super::backoff::Backoff;
use super::spectrum::{FFT_SIZE, PcmDecoder, SampleWindow, Smoother, SpectrumAnalyzer};
use crate::domain::{PcmFormat, SpectrumFrame};
use crate::repository::{Error, PcmSource};

pub const VISUALIZER_FPS: u32 = 20;
/// Sin datos durante este tiempo, se considera que no hay señal.
pub const SILENCE_TIMEOUT: Duration = Duration::from_millis(250);
const READ_BUFFER_BYTES: usize = 16 * 1024;

/// Lo que publica el visualizador.
#[derive(Debug, Clone, PartialEq)]
pub enum SpectrumState {
    Frame(SpectrumFrame),
    /// No se puede abrir la fuente; el motivo es para el usuario (`no existe`).
    Unavailable(String),
}

/// Antes del primer cuadro: nada que dibujar.
impl Default for SpectrumState {
    fn default() -> Self {
        Self::Frame(SpectrumFrame::default())
    }
}

pub struct Visualizer<S> {
    source: S,
    format: PcmFormat,
    backoff: Backoff,
    bands: watch::Receiver<usize>,
    states: watch::Sender<SpectrumState>,
}

/// Estado del bucle de lectura y análisis.
struct Analysis {
    decoder: PcmDecoder,
    window: SampleWindow,
    analyzer: SpectrumAnalyzer,
    smoother: Smoother,
    last_frame: Instant,
    last_data: Option<Instant>,
}

impl Analysis {
    fn new(format: PcmFormat) -> Self {
        Self {
            decoder: PcmDecoder::new(format),
            window: SampleWindow::new(FFT_SIZE),
            analyzer: SpectrumAnalyzer::new(format.sample_rate()),
            smoother: Smoother::default(),
            last_frame: Instant::now(),
            last_data: None,
        }
    }

    fn receive(&mut self, bytes: &[u8], now: Instant) {
        self.decoder.decode(bytes, &mut self.window);
        self.last_data = Some(now);
    }

    /// Sin señal, las barras caen hasta cero con su velocidad de caída.
    fn frame(&mut self, bands: usize, now: Instant) -> SpectrumFrame {
        let signal = self
            .last_data
            .is_some_and(|last| now.saturating_duration_since(last) < SILENCE_TIMEOUT);
        let targets = if signal {
            self.analyzer.levels(&self.window.latest(), bands)
        } else {
            vec![0.0; bands]
        };
        let elapsed = now.saturating_duration_since(self.last_frame);
        self.last_frame = now;
        self.smoother.update(&targets, elapsed);
        self.smoother.frame(signal)
    }
}

impl<S: PcmSource + Send> Visualizer<S> {
    pub fn new(
        source: S,
        format: PcmFormat,
        backoff: Backoff,
        bands: watch::Receiver<usize>,
        states: watch::Sender<SpectrumState>,
    ) -> Self {
        Self {
            source,
            format,
            backoff,
            bands,
            states,
        }
    }

    /// Corre hasta que nadie escuche. Tras un fin de archivo o un error, vuelve a
    /// abrir la fuente con espera creciente, sin dejar de publicar cuadros.
    pub async fn run(mut self) {
        let mut analysis = Analysis::new(self.format);
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
                        if !self.publish(SpectrumState::Unavailable(describe(&error))) {
                            return;
                        }
                    }
                },
                () = sleep_until(retry_at) => retry_at = None,
                now = frames.tick() => {
                    let frame = analysis.frame(*self.bands.borrow(), now);
                    if !unavailable && !self.publish(SpectrumState::Frame(frame)) {
                        return;
                    }
                }
            }
        }
    }

    /// `false` si ya nadie escucha.
    fn publish(&self, state: SpectrumState) -> bool {
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

/// Motivo legible de que la fuente no se pueda leer.
fn describe(error: &Error) -> String {
    match error {
        Error::Io(io_error) => match io_error.kind() {
            io::ErrorKind::NotFound => "no existe".to_owned(),
            io::ErrorKind::PermissionDenied => "sin permiso de lectura".to_owned(),
            _ => io_error.to_string(),
        },
        other => other.to_string(),
    }
}

/// Abre una fuente nueva cada vez que se lanza el visualizador.
type SourceOpener<S> = Box<dyn Fn() -> S + Send + Sync>;

/// Lanza el visualizador cuando hace falta (al entrar a la vista Espectro).
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

    pub fn launch(&self, bands: usize) -> RunningVisualizer {
        let (bands_sender, bands_receiver) = watch::channel(bands);
        let silent = SpectrumState::Frame(SpectrumFrame::silent(bands));
        let (states_sender, states_receiver) = watch::channel(silent);
        let visualizer = Visualizer::new(
            (self.open_source)(),
            self.format,
            self.backoff,
            bands_receiver,
            states_sender,
        );
        RunningVisualizer {
            task: tokio::spawn(visualizer.run()),
            bands: bands_sender,
            states: states_receiver,
        }
    }
}

/// El visualizador en marcha. Al soltarlo, la tarea se cancela y deja de leer.
pub struct RunningVisualizer {
    task: JoinHandle<()>,
    bands: watch::Sender<usize>,
    states: watch::Receiver<SpectrumState>,
}

impl RunningVisualizer {
    pub fn bands(&self) -> usize {
        *self.bands.borrow()
    }

    pub fn set_bands(&self, bands: usize) {
        self.bands.send_replace(bands);
    }

    /// Espera el próximo estado publicado; `None` si la tarea terminó.
    pub async fn next_state(&mut self) -> Option<SpectrumState> {
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

    use super::*;
    use crate::repository::Result;
    use crate::service::spectrum::sine_pcm;

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
        wanted: impl Fn(&SpectrumFrame) -> bool,
    ) -> SpectrumFrame {
        for _ in 0..FRAMES_TO_WAIT {
            if let Some(SpectrumState::Frame(frame)) = running.next_state().await
                && wanted(&frame)
            {
                return frame;
            }
        }
        panic!("no llegó el cuadro esperado");
    }

    #[tokio::test(start_paused = true)]
    async fn bars_follow_the_signal_and_fall_without_it() {
        let chunk = sine_pcm(1_000.0, 0.5, READ_BUFFER_BYTES / 4);
        let mut running = launcher(vec![Ok(chunk)]).launch(BANDS);
        let frame = frame_where(&mut running, |frame| frame.signal).await;
        assert_eq!(frame.levels.len(), BANDS);
        assert!(frame.levels.iter().any(|level| *level > 0.5));
        let frame = frame_where(&mut running, |frame| !frame.signal).await;
        assert!(frame.peaks.iter().any(|peak| *peak > 0.0));
        let frame = frame_where(&mut running, SpectrumFrame::is_flat).await;
        assert!(!frame.signal);
    }

    #[tokio::test(start_paused = true)]
    async fn a_missing_source_is_explained_and_retried() {
        let missing = io::Error::from(io::ErrorKind::NotFound);
        let chunk = sine_pcm(1_000.0, 0.5, 1_024);
        let mut running = launcher(vec![Err(missing.into()), Ok(chunk)]).launch(BANDS);
        assert_eq!(
            running.next_state().await,
            Some(SpectrumState::Unavailable("no existe".into()))
        );
        frame_where(&mut running, |frame| frame.signal).await;
    }

    #[tokio::test(start_paused = true)]
    async fn band_count_can_change_while_running() {
        let mut running = launcher(Vec::new()).launch(BANDS);
        running.set_bands(BANDS * 2);
        assert_eq!(running.bands(), BANDS * 2);
        frame_where(&mut running, |frame| frame.levels.len() == BANDS * 2).await;
    }
}
