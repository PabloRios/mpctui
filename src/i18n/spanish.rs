//! Textos en español.

use std::io;

use super::{Catalog, Message, Phrase, counted, seconds_with_decimal};
use crate::config::{
    COLORS_VARIABLE, ConfigError, GLYPHS_VARIABLE, PORT_VARIABLE, TIMEOUT_VARIABLE, color_choices,
    glyph_choices, language_choices, lyrics_source_choices, theme_choices,
    visualizer_style_choices,
};
use crate::domain::{
    InputError, MAX_VOLUME, PATH_SEPARATOR, PCM_BITS, PCM_MAX_CHANNELS, PCM_MAX_SAMPLE_RATE,
    PCM_MIN_SAMPLE_RATE,
};
use crate::repository::{Error, HttpFailure};

const DECIMAL_SEPARATOR: char = ',';

#[derive(Debug, Clone, Copy, Default)]
pub struct Spanish;

impl Catalog for Spanish {
    fn phrase(&self, phrase: Phrase) -> &'static str {
        match phrase {
            Phrase::Quit => "Salir",
            Phrase::ToggleHelp => "Mostrar u ocultar la ayuda",
            Phrase::GoToView => "Ir a la vista",
            Phrase::CycleVisualizer => "Visualizador / otra forma",
            Phrase::NextTheme => "Tema de colores siguiente",
            Phrase::SortQueue => "Ordenar la cola",
            Phrase::ShowSongInfo => "Información del tema",
            Phrase::CycleViews => "Vista siguiente / anterior",
            Phrase::MoveDown => "Bajar",
            Phrase::MoveUp => "Subir",
            Phrase::PageDown => "Bajar una página",
            Phrase::PageUp => "Subir una página",
            Phrase::GoToFirst => "Ir al principio",
            Phrase::GoToLast => "Ir al final",
            Phrase::Activate => "Reproducir / entrar / cargar",
            Phrase::Forward => "Entrar / columna siguiente",
            Phrase::Back => "Volver / columna anterior",
            Phrase::AddSelection => "Agregar (playlist: cargar)",
            Phrase::AddAll => "Agregar todos",
            Phrase::DeleteSelection => "Borrar (playlist: confirma)",
            Phrase::TogglePause => "Pausa / reanudar",
            Phrase::Stop => "Detener",
            Phrase::NextSong => "Tema siguiente",
            Phrase::PreviousSong => "Tema anterior",
            Phrase::VolumeUp => "Subir volumen",
            Phrase::VolumeDown => "Bajar volumen",
            Phrase::SeekForward => "Adelantar",
            Phrase::SeekBackward => "Retroceder",
            Phrase::ClearQueue => "Vaciar la cola (confirma)",
            Phrase::SaveQueue => "Guardar la cola como playlist",
            Phrase::Search => "Buscar",
            Phrase::Random => "Aleatorio",
            Phrase::Repeat => "Repetir",
            Phrase::CycleSingle => "Single: no, sí, una vez",
            Phrase::Consume => "Consumir",
            Phrase::ShowPlaying => "Ir al tema que suena",
            Phrase::MoveSong => "Mover el tema abajo / arriba",
            Phrase::AddToPlaylist => "Agregar a una playlist",
            Phrase::RetryLyrics => "Reintentar la descarga de la letra",
            Phrase::UpdateDatabase => "Actualizar la biblioteca",
            Phrase::NewPartition => "Nueva partición",
            Phrase::MoveOutput => "Mover salida a partición",
            Phrase::HelpNavigation => "Navegación",
            Phrase::HelpLists => "Cola y listas",
            Phrase::HelpPlayback => "Reproducción",
            Phrase::HelpModes => "Modos",
            Phrase::HelpOther => "Otros",
            Phrase::HelpTitle => "Ayuda",
            Phrase::HelpClose => "cualquier tecla cierra",
            Phrase::KeySpace => "Espacio",
            Phrase::KeyPageUp => "RePág",
            Phrase::KeyPageDown => "AvPág",
            Phrase::KeyHome => "Inicio",
            Phrase::KeyEnd => "Fin",
            Phrase::KeyBackspace => "Retroceso",
            Phrase::KeyDelete => "Supr",
            Phrase::KeyUp => "Arriba",
            Phrase::KeyDown => "Abajo",
            Phrase::KeyLeft => "Izq",
            Phrase::KeyRight => "Der",
            Phrase::HintPlay => "reproducir",
            Phrase::HintDelete => "borrar",
            Phrase::HintShowPlaying => "ir al actual",
            Phrase::HintSearch => "buscar",
            Phrase::HintHelp => "ayuda",
            Phrase::HintOpen => "entrar",
            Phrase::HintBack => "volver",
            Phrase::HintAdd => "agregar",
            Phrase::HintAddAlbum => "agregar álbum",
            Phrase::HintOpenOrAdd => "entrar o agregar",
            Phrase::HintParent => "subir",
            Phrase::HintAddAll => "agregar todos",
            Phrase::HintLoad => "cargar",
            Phrase::HintSaveQueue => "guardar cola",
            Phrase::HintMoveDown => "bajar",
            Phrase::HintMoveUp => "subir",
            Phrase::HintEdit => "editar",
            Phrase::HintRemove => "quitar",
            Phrase::HintToPlaylist => "a playlist",
            Phrase::HintOutputs => "salidas",
            Phrase::HintNewPartition => "nueva partición",
            Phrase::HintToggleOutput => "activar/desactivar",
            Phrase::HintMoveOutput => "mover",
            Phrase::HintUsePartition => "usar",
            Phrase::HintSort => "ordenar",
            Phrase::HintPause => "pausa",
            Phrase::HintNextStyle => "siguiente forma",
            Phrase::HintRetry => "reintentar",
            Phrase::HintPlayFromLine => "reproducir desde acá",
            Phrase::HintFollowSong => "seguir la canción",
            Phrase::SearchKeys => "Enter busca   Tab cambia el campo   Esc cancela",
            Phrase::SaveQueueTitle => "Guardar cola como playlist",
            Phrase::ConfirmTitle => "Confirmar",
            Phrase::NameLabel => "Nombre",
            Phrase::SaveKeys => "Enter guarda   Esc cancela",
            Phrase::NewPlaylistTitle => "Nueva playlist",
            Phrase::PickerTitle => "Agregar a playlist",
            Phrase::PickerKeys => "Enter elige   Esc cancela",
            Phrase::NewPlaylistChoice => "Nueva playlist",
            Phrase::NewPartitionTitle => "Nueva partición",
            Phrase::PartitionPickerTitle => "Mover a la partición",
            Phrase::SortPickerTitle => "Ordenar la cola por",
            Phrase::SortPickerKeys => "Espacio elige  Enter ordena  Esc cancela",
            Phrase::SongInfoTitle => "Información del tema",
            Phrase::SongInfoKeys => "j k desplazan   otra tecla cierra",
            Phrase::Yes => "sí",
            Phrase::No => "no",
            Phrase::ConfirmKey => "s",
            Phrase::ViewQueue => "Cola",
            Phrase::ViewLibrary => "Biblioteca",
            Phrase::ViewLibraryShort => "Bibl.",
            Phrase::ViewBrowse => "Directorios",
            Phrase::ViewBrowseShort => "Dir.",
            Phrase::ViewSearch => "Búsqueda",
            Phrase::ViewSearchShort => "Búsq.",
            Phrase::ViewPlaylists => "Playlists",
            Phrase::ViewPlaylistsShort => "Playl.",
            Phrase::ViewVisualizer => "Visual",
            Phrase::ViewLyrics => "Letra",
            Phrase::ViewOutputs => "Salidas",
            Phrase::ViewOutputsShort => "Sal.",
            Phrase::Disconnected => "sin conexión",
            Phrase::Connecting => "conectando",
            Phrase::Playing => "reproduciendo",
            Phrase::Paused => "en pausa",
            Phrase::Stopped => "detenido",
            Phrase::NothingPlaying => "sin reproducción",
            Phrase::Live => "en vivo",
            Phrase::VolumeUnavailable => "n/d",
            Phrase::VolumeLabel => "vol",
            Phrase::PositionLabel => "pos",
            Phrase::SongLabel => "tema",
            Phrase::AlbumLabel => "álbum",
            Phrase::QueueLabel => "cola",
            Phrase::ModeRandom => "aleatorio",
            Phrase::ModeRepeat => "repetir",
            Phrase::ModeSingle => "single",
            Phrase::ModeConsume => "consumir",
            Phrase::Unnamed => "(sin nombre)",
            Phrase::NoAlbum => "[sin álbum]",
            Phrase::TagAny => "Todo",
            Phrase::TagArtist => "Artista",
            Phrase::TagAlbumArtist => "Artista del álbum",
            Phrase::TagAlbum => "Álbum",
            Phrase::TagTitle => "Título",
            Phrase::TagDate => "Fecha",
            Phrase::TagOriginalDate => "Fecha original",
            Phrase::TagTrack => "Pista",
            Phrase::TagDisc => "Disco",
            Phrase::TagGenre => "Género",
            Phrase::TagComposer => "Compositor",
            Phrase::TagPerformer => "Intérprete",
            Phrase::TagConductor => "Director",
            Phrase::TagComment => "Comentario",
            Phrase::InfoFile => "Archivo",
            Phrase::InfoDuration => "Duración",
            Phrase::InfoFormat => "Formato",
            Phrase::InfoLastModified => "Modificado",
            Phrase::ColumnPosition => "pos",
            Phrase::ColumnDuration => "dur",
            Phrase::ColumnSong => "Tema",
            Phrase::PartitionsTitle => "Particiones",
            Phrase::SortArtist => "Artista",
            Phrase::SortYear => "Año",
            Phrase::SortAlbum => "Álbum",
            Phrase::SortDisc => "Disco",
            Phrase::SortTrack => "Pista",
            Phrase::SortTitle => "Título",
            Phrase::SortFile => "Archivo",
            Phrase::Mono => "mono",
            Phrase::Stereo => "estéreo",
            Phrase::FloatSamples => "flotante",
            Phrase::StyleBars => "Barras",
            Phrase::StyleMirror => "Espejo estéreo",
            Phrase::StyleWave => "Onda",
            Phrase::StyleWaterfall => "Cascada",
            Phrase::StyleVectorscope => "Vectorscopio",
            Phrase::StyleMeters => "Medidores",
            Phrase::VisualizerTitle => "Visualizador",
            Phrase::NoSignal => "sin señal",
            Phrase::FifoAddOutput => "Agregá una salida fifo en mpd.conf y reiniciá MPD:",
            Phrase::FifoOutputName => "visualizador",
            Phrase::FifoOtherPath => {
                "Si usás otra ruta o formato, configurá visualizer_fifo y visualizer_format."
            }
            Phrase::MetersDetail => "RMS y pico por canal",
            Phrase::Peak => "pico",
            Phrase::Now => "ahora",
            Phrase::Correlation => "correlación",
            Phrase::NoSong => "Sin tema en reproducción",
            Phrase::SearchingLyrics => "buscando letra",
            Phrase::Instrumental => "Instrumental",
            Phrase::UntitledHint => "Al tema le faltan el artista o el título en los tags.",
            Phrase::LastError => "Último error",
            Phrase::Synced => "sincronizada",
            Phrase::PlainText => "texto",
            Phrase::SourceCache => "caché",
            Phrase::SourceSongFolder => "carpeta del tema",
            Phrase::SourceTags => "etiquetas",
            Phrase::FieldLabel => "campo",
            Phrase::FieldHint => "Tab cambia",
            Phrase::ConnectionRecovered => "Conexión recuperada",
            Phrase::LyricsDisabled => "Letras desactivadas (lyrics.enabled = false)",
            Phrase::LibraryUpdateRequested => "Actualización de la biblioteca pedida",
            Phrase::QueueCleared => "Cola vaciada",
            Phrase::QueueSorted => "Cola ordenada",
            Phrase::NoVolumeControl => "El servidor no tiene control de volumen",
            Phrase::DefaultPartitionUndeletable => "La partición default no se puede borrar",
            Phrase::ActivePartitionUndeletable => "La partición en uso no se puede borrar",
            Phrase::ThemesNeedColors => "Los temas requieren 256 colores",
            Phrase::TerminalTooSmall => "La terminal es muy chica.",
        }
    }

    fn message(&self, message: &Message<'_>) -> String {
        match *message {
            Message::SongCount(count) => songs(count),
            Message::EntryCount(count) => counted(count, "entrada", "entradas"),
            Message::OutputCount(count) => counted(count, "salida", "salidas"),
            Message::ChannelCount(count) => format!("{count} canales"),
            Message::ResultsInAlbums { results, albums } => format!(
                "{} en {}",
                counted(results, "resultado", "resultados"),
                counted(albums, "álbum", "álbumes")
            ),
            Message::UpdatingLibrary { elapsed } => format!("actualizando biblioteca {elapsed}"),
            Message::RetryIn { countdown } => format!("reintento en {countdown}"),
            Message::Attempt(attempt) => format!("(intento {attempt})"),
            Message::QueueSummary(count) => format!("cola: {}", songs(count)),
            Message::ConfirmClearQueue(count) => {
                format!("¿Vaciar la cola? Se quitan {}.", songs(count))
            }
            Message::ConfirmRemovePlaylist(name) => format!("¿Borrar la playlist «{name}»?"),
            Message::ConfirmRemovePartition { name, outputs } => match outputs {
                0 => format!("¿Borrar la partición «{name}»?"),
                1 => format!("¿Borrar la partición «{name}»? Su salida vuelve a default."),
                many => {
                    format!("¿Borrar la partición «{name}»? Sus {many} salidas vuelven a default.")
                }
            },
            Message::TerminalSize {
                minimum_width,
                minimum_height,
                width,
                height,
            } => format!("Mínimo: {minimum_width} x {minimum_height}. Actual: {width} x {height}."),
            Message::EmptyQueue { key } => {
                format!("La cola está vacía. {key} abre la biblioteca para agregar temas.")
            }
            Message::NoPlaylists { key } => {
                format!("No hay playlists guardadas. {key} guarda la cola como playlist.")
            }
            Message::NoOutputs { key } => {
                format!("Sin salidas. Elegí una en otra partición y traela con {key}.")
            }
            Message::SearchHint { key } => format!("{key} busca en la biblioteca"),
            Message::NoResults { query } => format!("Sin resultados para \"{query}\""),
            Message::LyricsNotFound { title } => {
                format!("No se encontró la letra para «{title}»")
            }
            Message::CannotRead { source, error } => {
                format!("No se puede leer {source} ({}).", unreadable(error))
            }
            Message::BarsDetail { bands, low, high } => {
                format!("{bands} bandas  {low} - {high}")
            }
            Message::MirrorDetail { bands } => {
                format!("{bands} bandas por canal  L arriba, R abajo")
            }
            Message::WaveDetail { span, synced } => {
                let sync = if synced { "sincronizado" } else { "libre" };
                format!("{} ms  {sync}", span.as_millis())
            }
            Message::WaterfallDetail { bands, seconds } => {
                format!("{bands} bandas  últimos {seconds} s")
            }
            Message::VectorscopeDetail { points } => format!("M/S  {points} muestras"),
            Message::MetersLegend {
                window,
                peak_glyph,
                hold,
            } => format!(
                "barra = RMS ({} ms)   {peak_glyph} = pico retenido {}",
                window.as_millis(),
                seconds_with_decimal(hold, DECIMAL_SEPARATOR)
            ),
            Message::VisualizerChanged { style, key } => {
                format!("Visualizador: {style} ({key}: siguiente forma)")
            }
            Message::ThemeChanged { theme, key } => format!("Tema: {theme} ({key}: siguiente)"),
            Message::ThemeNeedsColors { theme } => {
                format!("Tema {theme}: requiere 256 colores, se usa terminal")
            }
            Message::ConnectionLost(error) => {
                format!("Sin conexión con MPD: {}", failure(error))
            }
            Message::AddedToQueue(what) => format!("Agregado a la cola: {what}"),
            Message::SongsAdded(count) => format!("Agregados a la cola: {}", songs(count)),
            Message::ResultsFound(count) => counted(count, "resultado", "resultados"),
            Message::QueueSaved(name) => format!("Cola guardada como «{name}»"),
            Message::PlaylistDeleted(name) => format!("Playlist «{name}» borrada"),
            Message::PlaylistLoaded(name) => format!("Playlist «{name}» agregada a la cola"),
            Message::AddedToPlaylist { what, playlist } => {
                format!("Agregado a «{playlist}»: {what}")
            }
            Message::RemovedFromPlaylist { song, playlist } => {
                format!("Quitado de «{playlist}»: {song}")
            }
            Message::LibraryUpdated { elapsed } => format!("Biblioteca actualizada ({elapsed})"),
            Message::NoOtherPartition { key } => {
                format!("No hay otra partición: {key} crea una")
            }
            Message::PartitionCreated(name) => format!("Partición «{name}» creada"),
            Message::UsingPartition(name) => format!("mpctui usa ahora la partición «{name}»"),
            Message::PartitionDeleted(name) => format!("Partición «{name}» borrada"),
            Message::OutputMoved { output, partition } => {
                format!("«{output}» movida a «{partition}»")
            }
            Message::OutputToggled { output, enabled } => {
                let state = if enabled { "activada" } else { "desactivada" };
                format!("Salida «{output}» {state}")
            }
            Message::Failure(error) => failure(error),
            Message::Input(error) => input(error),
            Message::Config(error) => config(error),
            Message::ConnectFailed { address, error } => {
                format!("no se pudo conectar a MPD en {address}: {}", failure(error))
            }
        }
    }
}

fn songs(count: usize) -> String {
    counted(count, "tema", "temas")
}

/// Por qué no se puede leer la salida fifo.
fn unreadable(error: &Error) -> String {
    match error {
        Error::Io(io) if io.kind() == io::ErrorKind::NotFound => "no existe".to_owned(),
        Error::Io(io) if io.kind() == io::ErrorKind::PermissionDenied => {
            "sin permiso de lectura".to_owned()
        }
        Error::Io(io) => io.to_string(),
        other => failure(other),
    }
}

fn failure(error: &Error) -> String {
    match error {
        Error::Io(io) => format!("error de E/S: {io}"),
        Error::Ack(ack) => format!("MPD: {ack}"),
        Error::Protocol(detail) => format!("respuesta inválida de MPD: {detail}"),
        Error::Closed => "la conexión con MPD se cerró".to_owned(),
        Error::Timeout => "MPD no respondió a tiempo".to_owned(),
        Error::Http { service, failure } => format!("{service}: {}", http(failure)),
    }
}

fn http(failure: &HttpFailure) -> String {
    match failure {
        HttpFailure::Timeout => "no respondió a tiempo".to_owned(),
        HttpFailure::Connect => "no se pudo conectar".to_owned(),
        HttpFailure::InvalidResponse => "respuesta inválida".to_owned(),
        HttpFailure::Status(code) => format!("respondió {code}"),
        HttpFailure::Other(detail) => detail.clone(),
    }
}

fn input(error: &InputError) -> String {
    match error {
        InputError::VolumeOutOfRange(value) => {
            format!("el volumen debe estar entre 0 y {MAX_VOLUME}; se recibió {value}")
        }
        InputError::EmptyText => "el texto no puede estar vacío".to_owned(),
        InputError::ControlCharacter => {
            "el texto no puede contener saltos de línea ni caracteres de control".to_owned()
        }
        InputError::PathSeparatorInName => {
            format!("el nombre de la playlist no puede contener '{PATH_SEPARATOR}'")
        }
        InputError::InvalidPartitionName => {
            "el nombre de la partición solo admite letras y dígitos ASCII, '-' y '_'".to_owned()
        }
        InputError::MalformedPcmFormat(value) => format!(
            "el formato PCM debe ser frecuencia:bits:canales (como \"44100:16:2\"), no {value:?}"
        ),
        InputError::SampleRateOutOfRange(rate) => format!(
            "la frecuencia debe estar entre {PCM_MIN_SAMPLE_RATE} y {PCM_MAX_SAMPLE_RATE} Hz; se recibió {rate}"
        ),
        InputError::UnsupportedBits(bits) => {
            format!("solo se admiten muestras de {PCM_BITS} bits; se recibió {bits}")
        }
        InputError::UnsupportedChannels(channels) => {
            format!("solo se admiten 1 o {PCM_MAX_CHANNELS} canales; se recibió {channels}")
        }
    }
}

fn config(error: &ConfigError) -> String {
    match error {
        ConfigError::Read { path, source } => {
            format!("no se pudo leer {}: {source}", path.display())
        }
        ConfigError::Parse { path, source } => {
            format!("configuración inválida en {}: {source}", path.display())
        }
        ConfigError::InvalidPort(value) => {
            format!("{PORT_VARIABLE} no es un puerto válido: {value:?}")
        }
        ConfigError::EmptyHost => "el host de MPD no puede estar vacío".to_owned(),
        ConfigError::InvalidTimeout(value) => format!(
            "{TIMEOUT_VARIABLE} debe ser una cantidad entera de segundos mayor que 0, no {value:?}"
        ),
        ConfigError::InvalidPassword(error) => {
            format!("contraseña de MPD inválida: {}", input(error))
        }
        ConfigError::InvalidGlyphs(setting) => format!(
            "{GLYPHS_VARIABLE} debe ser {}, no {:?}",
            glyph_choices(),
            setting.0
        ),
        ConfigError::InvalidLanguage(value) => {
            format!("language debe ser {}, no {value:?}", language_choices())
        }
        ConfigError::InvalidTheme(value) => {
            format!("theme debe ser {}, no {value:?}", theme_choices())
        }
        ConfigError::InvalidColors(value) => format!(
            "colors (o {COLORS_VARIABLE}) debe ser {}, no {value:?}",
            color_choices()
        ),
        ConfigError::EmptyVisualizerFifo => "visualizer_fifo no puede estar vacío".to_owned(),
        ConfigError::InvalidVisualizerFormat(error) => {
            format!("visualizer_format inválido: {}", input(error))
        }
        ConfigError::InvalidVisualizerStyle(value) => format!(
            "visualizer_style debe ser {}, no {value:?}",
            visualizer_style_choices()
        ),
        ConfigError::StereoStyleWithMonoFifo(value) => format!(
            "visualizer_style = {value:?} necesita dos canales y visualizer_format tiene uno"
        ),
        ConfigError::UnknownLyricsSource(value) => format!(
            "lyrics.sources admite {}, no {value:?}",
            lyrics_source_choices()
        ),
        ConfigError::RepeatedLyricsSource(value) => format!("lyrics.sources repite {value:?}"),
        ConfigError::EmptyLyricsSources => {
            "lyrics.sources no puede estar vacía con las letras activadas".to_owned()
        }
        ConfigError::SongFolderWithoutMusicDirectory => {
            "lyrics.sources incluye \"song_folder\" pero falta lyrics.music_directory".to_owned()
        }
        ConfigError::EmptyLyricsPath(field) => format!("lyrics.{field} no puede estar vacío"),
        ConfigError::UnknownHomeDirectory(field) => {
            format!("no se pudo saber el directorio del usuario para lyrics.{field}")
        }
    }
}
