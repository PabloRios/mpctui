//! Textos que ve el usuario, en cada idioma. Es el único lugar con palabras de la
//! interfaz: el resto del código pide un texto por su clave (`Phrase`) o arma un
//! `Message` con sus datos, y el catálogo del idioma activo (inyectado desde el
//! composition root, como los glifos) elige las palabras. Cada idioma es un `match`
//! exhaustivo: si falta una traducción, no compila.
//!
//! Las capas internas no producen texto para el usuario: devuelven datos o errores
//! tipados y acá se los explica. El `Display` de los errores queda para el log.

mod english;
mod spanish;

use std::fmt;
use std::time::Duration;

use crate::config::{ConfigError, Language};
use crate::domain::InputError;
use crate::repository::Error;

pub use self::english::English;
pub use self::spanish::Spanish;

/// Un idioma: textos fijos y textos con datos.
pub trait Catalog: fmt::Debug + Sync {
    fn phrase(&self, phrase: Phrase) -> &'static str;
    fn message(&self, message: &Message<'_>) -> String;
}

/// El catálogo de cada idioma.
pub fn catalog(language: Language) -> &'static dyn Catalog {
    match language {
        Language::English => &English,
        Language::Spanish => &Spanish,
    }
}

/// Textos fijos, agrupados por dónde aparecen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phrase {
    // Descripciones de los atajos en la ayuda.
    Quit,
    ToggleHelp,
    GoToView,
    CycleVisualizer,
    NextView,
    PreviousView,
    MoveDown,
    MoveUp,
    PageDown,
    PageUp,
    GoToFirst,
    GoToLast,
    Activate,
    Forward,
    Back,
    AddSelection,
    AddAll,
    DeleteSelection,
    TogglePause,
    Stop,
    NextSong,
    PreviousSong,
    VolumeUp,
    VolumeDown,
    SeekForward,
    SeekBackward,
    ClearQueue,
    SaveQueue,
    Search,
    Random,
    Repeat,
    CycleSingle,
    Consume,
    ShowPlaying,
    RetryLyrics,
    // Secciones y marco de la ayuda.
    HelpNavigation,
    HelpLists,
    HelpPlayback,
    HelpModes,
    HelpOther,
    HelpTitle,
    HelpClose,
    // Nombres de teclas. Las flechas, solo cuando el juego de glifos no las dibuja.
    KeySpace,
    KeyPageUp,
    KeyPageDown,
    KeyHome,
    KeyEnd,
    KeyBackspace,
    KeyDelete,
    KeyUp,
    KeyDown,
    KeyLeft,
    KeyRight,
    // Atajos sugeridos en la línea inferior.
    HintPlay,
    HintDelete,
    HintShowPlaying,
    HintSearch,
    HintHelp,
    HintOpen,
    HintBack,
    HintAdd,
    HintAddAlbum,
    HintOpenOrAdd,
    HintParent,
    HintAddAll,
    HintLoad,
    HintSaveQueue,
    HintPause,
    HintNextStyle,
    HintRetry,
    HintPlayFromLine,
    HintFollowSong,
    /// Teclas de la línea de búsqueda.
    SearchKeys,
    // Diálogos.
    SaveQueueTitle,
    ConfirmTitle,
    NameLabel,
    SaveKeys,
    Yes,
    No,
    /// La tecla que se muestra para confirmar (se aceptan las de los dos idiomas).
    ConfirmKey,
    // Pestañas; las cortas son para el modo compacto.
    ViewQueue,
    ViewLibrary,
    ViewLibraryShort,
    ViewBrowse,
    ViewBrowseShort,
    ViewSearch,
    ViewSearchShort,
    ViewPlaylists,
    ViewPlaylistsShort,
    ViewVisualizer,
    ViewLyrics,
    // Conexión en el encabezado.
    Disconnected,
    Connecting,
    // Panel "ahora suena".
    Playing,
    Paused,
    Stopped,
    NothingPlaying,
    /// Dentro de la barra de un stream, que no tiene duración.
    Live,
    VolumeUnavailable,
    VolumeLabel,
    PositionLabel,
    SongLabel,
    AlbumLabel,
    QueueLabel,
    ModeRandom,
    ModeRepeat,
    ModeSingle,
    ModeConsume,
    // Datos de las listas.
    Unnamed,
    NoAlbum,
    TagAny,
    TagArtist,
    TagAlbumArtist,
    TagAlbum,
    TagTitle,
    ColumnPosition,
    ColumnDuration,
    ColumnSong,
    // Formato de audio.
    Mono,
    Stereo,
    FloatSamples,
    // Visualizador.
    StyleBars,
    StyleMirror,
    StyleWave,
    StyleWaterfall,
    StyleVectorscope,
    StyleMeters,
    VisualizerTitle,
    NoSignal,
    FifoAddOutput,
    /// El `name` de la salida fifo del ejemplo de `mpd.conf`.
    FifoOutputName,
    FifoOtherPath,
    MetersDetail,
    Peak,
    Now,
    Correlation,
    // Letra.
    NoSong,
    SearchingLyrics,
    Instrumental,
    UntitledHint,
    LastError,
    Synced,
    PlainText,
    SourceCache,
    SourceSongFolder,
    SourceTags,
    // Búsqueda.
    FieldLabel,
    FieldHint,
    // Avisos.
    ConnectionRecovered,
    LyricsDisabled,
    QueueCleared,
    NoVolumeControl,
    TerminalTooSmall,
}

/// Textos con datos: cada idioma decide el orden de las palabras y los plurales.
#[derive(Debug, Clone, Copy)]
pub enum Message<'a> {
    // Cantidades.
    SongCount(usize),
    EntryCount(usize),
    ChannelCount(u8),
    ResultsInAlbums {
        results: usize,
        albums: usize,
    },
    // Encabezado.
    UpdatingLibrary {
        elapsed: &'a str,
    },
    RetryIn {
        countdown: &'a str,
    },
    Attempt(u32),
    QueueSummary(usize),
    // Diálogos.
    ConfirmClearQueue(usize),
    ConfirmRemovePlaylist(&'a str),
    TerminalSize {
        minimum_width: u16,
        minimum_height: u16,
        width: u16,
        height: u16,
    },
    // Vistas vacías y resúmenes.
    EmptyQueue {
        key: &'a str,
    },
    NoPlaylists {
        key: &'a str,
    },
    SearchHint {
        key: &'a str,
    },
    NoResults {
        query: &'a str,
    },
    LyricsNotFound {
        title: &'a str,
    },
    // Visualizador.
    CannotRead {
        source: &'a str,
        error: &'a Error,
    },
    BarsDetail {
        bands: usize,
        low: &'a str,
        high: &'a str,
    },
    MirrorDetail {
        bands: usize,
    },
    WaveDetail {
        span: Duration,
        synced: bool,
    },
    WaterfallDetail {
        bands: usize,
        seconds: usize,
    },
    VectorscopeDetail {
        points: usize,
    },
    MetersLegend {
        window: Duration,
        peak_glyph: &'a str,
        hold: Duration,
    },
    VisualizerChanged {
        style: &'a str,
        key: &'a str,
    },
    // Avisos.
    ConnectionLost(&'a Error),
    AddedToQueue(&'a str),
    SongsAdded(usize),
    ResultsFound(usize),
    QueueSaved(&'a str),
    PlaylistDeleted(&'a str),
    PlaylistLoaded(&'a str),
    LibraryUpdated {
        elapsed: &'a str,
    },
    // Errores.
    Failure(&'a Error),
    Input(&'a InputError),
    Config(&'a ConfigError),
    ConnectFailed {
        address: &'a str,
        error: &'a Error,
    },
}

/// `1 song` / `10 songs`: en inglés y en español, el singular es solo para uno.
fn counted(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

/// Segundos con un decimal y el separador del idioma: `1.5 s` o `1,5 s`.
fn seconds_with_decimal(duration: Duration, separator: char) -> String {
    let seconds = format!("{:.1}", duration.as_secs_f32()).replace('.', &separator.to_string());
    format!("{seconds} s")
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::repository::HttpFailure;

    const CATALOGS: [&dyn Catalog; 2] = [&English, &Spanish];

    /// Una muestra de cada grupo de claves: ninguna puede quedar vacía.
    const SAMPLE_PHRASES: [Phrase; 12] = [
        Phrase::Quit,
        Phrase::HelpTitle,
        Phrase::KeySpace,
        Phrase::HintHelp,
        Phrase::ConfirmKey,
        Phrase::ViewQueue,
        Phrase::Playing,
        Phrase::TagAny,
        Phrase::StyleBars,
        Phrase::NoSong,
        Phrase::FieldHint,
        Phrase::TerminalTooSmall,
    ];

    #[test]
    fn english_is_the_default_language() {
        assert_eq!(catalog(Language::default()).phrase(Phrase::Quit), "Quit");
        assert_eq!(catalog(Language::Spanish).phrase(Phrase::Quit), "Salir");
    }

    #[test]
    fn no_phrase_is_empty() {
        for texts in CATALOGS {
            for phrase in SAMPLE_PHRASES {
                assert!(
                    !texts.phrase(phrase).trim().is_empty(),
                    "{texts:?} {phrase:?}"
                );
            }
        }
    }

    #[test]
    fn counts_follow_each_language_plural() {
        assert_eq!(English.message(&Message::SongCount(1)), "1 song");
        assert_eq!(English.message(&Message::SongCount(3)), "3 songs");
        assert_eq!(Spanish.message(&Message::SongCount(1)), "1 tema");
        assert_eq!(Spanish.message(&Message::EntryCount(2)), "2 entradas");
        assert_eq!(
            English.message(&Message::ResultsInAlbums {
                results: 1,
                albums: 2
            }),
            "1 result in 2 albums"
        );
        assert_eq!(
            Spanish.message(&Message::ResultsInAlbums {
                results: 3,
                albums: 1
            }),
            "3 resultados en 1 álbum"
        );
    }

    #[test]
    fn decimals_use_the_separator_of_the_language() {
        let legend = |texts: &dyn Catalog| {
            texts.message(&Message::MetersLegend {
                window: Duration::from_millis(300),
                peak_glyph: "|",
                hold: Duration::from_millis(1_500),
            })
        };
        assert!(legend(&English).ends_with("1.5 s"), "{}", legend(&English));
        assert!(legend(&Spanish).ends_with("1,5 s"), "{}", legend(&Spanish));
    }

    #[test]
    fn errors_are_explained_in_the_language() {
        let missing = Error::from(io::Error::from(io::ErrorKind::NotFound));
        let cannot_read = |texts: &dyn Catalog| {
            texts.message(&Message::CannotRead {
                source: "/tmp/mpd.fifo",
                error: &missing,
            })
        };
        assert!(cannot_read(&English).contains("does not exist"));
        assert!(cannot_read(&Spanish).contains("no existe"));
        let timeout = Error::Http {
            service: "LRCLIB",
            failure: HttpFailure::Timeout,
        };
        assert_eq!(
            English.message(&Message::Failure(&timeout)),
            "LRCLIB: timed out"
        );
        assert_eq!(
            Spanish.message(&Message::Failure(&Error::Closed)),
            Error::Closed.to_string()
        );
        let input = InputError::EmptyText;
        assert_eq!(
            English.message(&Message::Input(&input)),
            "the text cannot be empty"
        );
        let config = ConfigError::InvalidLanguage("fr".into());
        assert_eq!(
            English.message(&Message::Config(&config)),
            "language must be one of \"en\", \"es\", not \"fr\""
        );
    }
}
