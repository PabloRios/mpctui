//! Composition root: el único lugar que instancia colaboradores y los conecta.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::config::{GlyphMode, Settings};
use crate::domain::Password;
use crate::repository::mpd::{Address, MpdClient, MpdConnector, ReconnectingTransport};
use crate::repository::{FifoPcmSource, Result};
use crate::service::{
    Backoff, ChangeWatcher, LibraryService, PlaybackService, PlaylistService, QueueService,
    VisualizerLauncher,
};
use crate::tui::appearance::{Appearance, ProgressBars};
use crate::tui::glyphs::{self, Glyphs};
use crate::tui::{App, Backend, Services};

const WATCHER_FIRST_RETRY: Duration = Duration::from_secs(1);
const WATCHER_MAXIMUM_RETRY: Duration = Duration::from_secs(30);
/// El FIFO se reabre rápido: al volver de una pausa, el espectro tiene que reaparecer.
const FIFO_FIRST_RETRY: Duration = Duration::from_millis(50);
const FIFO_MAXIMUM_RETRY: Duration = Duration::from_secs(1);

/// Repositorio MPD con reconexión automática.
pub type MpdRepository = MpdClient<ReconnectingTransport<MpdConnector>>;

/// Todos los repositorios de la app son MPD.
pub struct MpdBackend;

impl Backend for MpdBackend {
    type Player = MpdRepository;
    type Queue = MpdRepository;
    type Library = MpdRepository;
    type Playlists = MpdRepository;
    type Pcm = FifoPcmSource;
}

/// Abre una conexión (autenticada si hay contraseña) que se reabre sola si se corta.
pub async fn connect_mpd(address: &Address, password: Option<&Password>) -> Result<MpdRepository> {
    let connector = MpdConnector::new(address.clone(), password.cloned());
    Ok(MpdClient::new(
        ReconnectingTransport::connect(connector).await?,
    ))
}

/// Arma la app completa. Cada servicio tiene su propia conexión, así ninguno espera a
/// otro, y el vigilante de cambios corre en su propia tarea.
pub async fn build_app(settings: &Settings) -> Result<App<MpdBackend>> {
    let connect = || connect_mpd(&settings.address, settings.password.as_ref());
    let services = Services {
        playback: PlaybackService::new(connect().await?),
        queue: QueueService::new(connect().await?),
        library: LibraryService::new(connect().await?),
        playlists: PlaylistService::new(connect().await?),
        visualizer: visualizer_launcher(settings),
    };
    let (sender, receiver) = mpsc::unbounded_channel();
    let backoff = Backoff::new(WATCHER_FIRST_RETRY, WATCHER_MAXIMUM_RETRY);
    let watcher = ChangeWatcher::new(connect().await?, sender, backoff);
    tokio::spawn(watcher.run());
    let appearance = Appearance {
        glyphs: glyphs_for(settings.glyph_mode),
        progress_bars: ProgressBars {
            album: settings.album_progress,
            queue: settings.queue_progress,
        },
        visualizer_style: settings.visualizer_style,
    };
    let server = settings.address.to_string();
    Ok(App::new(services, receiver, appearance, server))
}

/// El visualizador abre el FIFO recién cuando se lo lanza.
pub fn visualizer_launcher(settings: &Settings) -> VisualizerLauncher<FifoPcmSource> {
    let fifo = settings.visualizer_fifo.clone();
    let label = fifo.display().to_string();
    VisualizerLauncher::new(
        move || FifoPcmSource::new(fifo.clone()),
        label,
        settings.visualizer_format,
        Backoff::new(FIFO_FIRST_RETRY, FIFO_MAXIMUM_RETRY),
    )
}

fn glyphs_for(mode: GlyphMode) -> &'static Glyphs {
    match mode {
        GlyphMode::Unicode => &glyphs::UNICODE,
        GlyphMode::Ascii => &glyphs::ASCII,
    }
}
