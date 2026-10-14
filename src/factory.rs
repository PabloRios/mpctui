//! Composition root: el único lugar que instancia colaboradores y los conecta.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::config::{GlyphMode, Settings};
use crate::i18n;
use crate::repository::mpd::{
    Address, ConnectionSettings, MpdClient, MpdConnector, PartitionConnector,
    ReconnectingTransport, SharedPartition, first_reachable,
};
use crate::repository::{FifoPcmSource, LRCLIB_URL, LocalLyricsFiles, LrclibProvider, Result};
use crate::service::{
    Backoff, LibraryService, LyricsLookup, LyricsService, OutputService, PlaybackService,
    PlaylistService, QueueService, ServerEvent, VisualizerLauncher, WatcherLauncher,
};
use crate::tui::appearance::{Appearance, ProgressBars};
use crate::tui::glyphs::{self, Glyphs};
use crate::tui::theme::Theme;
use crate::tui::{App, Backend, Services};

const WATCHER_FIRST_RETRY: Duration = Duration::from_secs(1);
const WATCHER_MAXIMUM_RETRY: Duration = Duration::from_secs(30);
/// El FIFO se reabre rápido: al volver de una pausa, el espectro tiene que reaparecer.
const FIFO_FIRST_RETRY: Duration = Duration::from_millis(50);
const FIFO_MAXIMUM_RETRY: Duration = Duration::from_secs(1);

/// Repositorio MPD con reconexión automática.
pub type MpdRepository = MpdClient<ReconnectingTransport<MpdConnector>>;
/// Repositorio MPD que además sigue la partición que usa mpctui.
pub type FollowingRepository = MpdClient<ReconnectingTransport<PartitionConnector<MpdConnector>>>;

/// Todos los repositorios de la app son MPD. Siguen la partición en uso los que dependen
/// de ella (reproducción, cola, playlists, que cargan y guardan la cola, y vigilancia);
/// la biblioteca es la misma en todas y la administración elige la partición en cada
/// comando.
pub struct MpdBackend;

impl Backend for MpdBackend {
    type Player = FollowingRepository;
    type Queue = FollowingRepository;
    type Library = MpdRepository;
    type Playlists = FollowingRepository;
    type Outputs = MpdRepository;
    type Selection = SharedPartition;
    type Changes = FollowingRepository;
    type Pcm = FifoPcmSource;
}

/// Abre una conexión (autenticada si hay contraseña) que se reabre sola si se corta.
pub async fn connect_mpd(
    address: &Address,
    connection: &ConnectionSettings,
) -> Result<MpdRepository> {
    let connector = MpdConnector::new(address.clone(), connection.clone());
    Ok(MpdClient::new(
        ReconnectingTransport::connect(connector).await?,
    ))
}

/// Abre una conexión que entra a la partición elegida en `partition` antes de cada
/// comando, también al reconectar.
pub async fn connect_following(
    address: &Address,
    connection: &ConnectionSettings,
    partition: &SharedPartition,
) -> Result<FollowingRepository> {
    let connector = following_connector(address, connection, partition);
    Ok(MpdClient::new(
        ReconnectingTransport::connect(connector).await?,
    ))
}

fn following_connector(
    address: &Address,
    connection: &ConnectionSettings,
    partition: &SharedPartition,
) -> PartitionConnector<MpdConnector> {
    let connector = MpdConnector::new(address.clone(), connection.clone());
    PartitionConnector::new(connector, partition.clone())
}

/// Arma la app completa. MPD se busca una vez, en las direcciones candidatas, y la
/// primera que responde queda para toda la sesión. Cada servicio tiene su propia
/// conexión, así ninguno espera a otro, y el vigilante de cambios corre en su propia
/// tarea (la lanza la app).
pub async fn build_app(settings: &Settings) -> Result<App<MpdBackend>> {
    let connection = &settings.connection;
    let address = &first_reachable(&settings.addresses, connection).await?;
    let partition = SharedPartition::default();
    let follow = || connect_following(address, connection, &partition);
    let (sender, receiver) = mpsc::unbounded_channel();
    let services = Services {
        playback: PlaybackService::new(follow().await?),
        queue: QueueService::new(follow().await?),
        library: LibraryService::new(connect_mpd(address, connection).await?),
        playlists: PlaylistService::new(follow().await?),
        outputs: OutputService::new(connect_mpd(address, connection).await?, partition.clone()),
        watcher: watcher_launcher(address, connection, &partition, sender),
        visualizer: visualizer_launcher(settings),
        lyrics: lyrics_lookup(settings, address).await?,
        lyrics_in_background: settings.lyrics.fetch_in_background,
    };
    let appearance = Appearance {
        glyphs: glyphs_for(settings.glyph_mode),
        texts: i18n::catalog(settings.language),
        theme: Theme::resolve(settings.theme, settings.color_depth),
        progress_bars: ProgressBars {
            album: settings.album_progress,
            queue: settings.queue_progress,
        },
        visualizer_style: settings.visualizer_style,
        lyrics_line: settings.lyrics.enabled && settings.lyrics.now_playing_line,
    };
    let server = address.to_string();
    Ok(App::new(services, receiver, appearance, server))
}

/// Cada vigilante abre su conexión con el primer `idle`, ya en la partición en uso.
fn watcher_launcher(
    address: &Address,
    connection: &ConnectionSettings,
    partition: &SharedPartition,
    events: mpsc::UnboundedSender<ServerEvent>,
) -> WatcherLauncher<FollowingRepository> {
    let address = address.clone();
    let connection = connection.clone();
    let partition = partition.clone();
    let backoff = Backoff::new(WATCHER_FIRST_RETRY, WATCHER_MAXIMUM_RETRY);
    WatcherLauncher::new(
        move || {
            let connector = following_connector(&address, &connection, &partition);
            MpdClient::new(ReconnectingTransport::lazy(connector))
        },
        events,
        backoff,
    )
}

/// Con las letras activadas, su búsqueda corre en su propia tarea y con su propia
/// conexión (para `readcomments`).
async fn lyrics_lookup(settings: &Settings, address: &Address) -> Result<Option<LyricsLookup>> {
    let lyrics = &settings.lyrics;
    if !lyrics.enabled {
        return Ok(None);
    }
    let files = LocalLyricsFiles::new(lyrics.directory.clone(), lyrics.music_directory.clone());
    let embedded = connect_mpd(address, &settings.connection).await?;
    let provider = LrclibProvider::new(LRCLIB_URL)?;
    let service = LyricsService::new(embedded, files, provider, lyrics.sources.clone());
    Ok(Some(LyricsLookup::spawn(service)))
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
