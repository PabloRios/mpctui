//! Service: lógica de negocio sobre los contratos del repositorio.

mod backoff;
mod changes;
mod database_update;
mod filters;
mod library;
mod playback;
mod playlists;
mod progress;
mod queue;
mod spectrum;
mod visualizer;

pub use backoff::Backoff;
pub use changes::{ChangeWatcher, ServerEvent};
pub use database_update::{UpdateTransition, update_transition};
pub use library::{LIBRARY_ROOT, LibraryService};
pub use playback::{PlaybackService, PlayerSnapshot, STATUS_POLL_INTERVAL};
pub use playlists::PlaylistService;
pub use progress::PlaybackClock;
pub use queue::QueueService;
pub use spectrum::{
    CEIL_DB, FFT_SIZE, FLOOR_DB, PcmDecoder, SampleWindow, Smoother, SpectrumAnalyzer,
};
pub use visualizer::{
    RunningVisualizer, SILENCE_TIMEOUT, SpectrumState, VISUALIZER_FPS, Visualizer,
    VisualizerLauncher,
};
