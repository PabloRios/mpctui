//! Modelos del dominio compartidos por todas las capas.

use std::time::Duration;

use super::audio::AudioFormat;
use super::inputs::{PATH_SEPARATOR, Volume};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    Play,
    Pause,
    #[default]
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SingleMode {
    #[default]
    Off,
    On,
    Oneshot,
}

impl SingleMode {
    /// Siguiente modo en el ciclo Off → On → Oneshot → Off.
    pub fn next(self) -> Self {
        match self {
            SingleMode::Off => SingleMode::On,
            SingleMode::On => SingleMode::Oneshot,
            SingleMode::Oneshot => SingleMode::Off,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// `None` cuando el servidor no tiene mixer.
    pub volume: Option<Volume>,
    pub repeat: bool,
    pub random: bool,
    pub single: SingleMode,
    pub consume: bool,
    pub queue_length: u32,
    pub state: PlayState,
    pub song_position: Option<u32>,
    pub elapsed: Option<Duration>,
    pub duration: Option<Duration>,
    /// Solo mientras hay reproducción.
    pub audio: Option<AudioFormat>,
    /// En kbps, instantáneo (varía en VBR); `None` si se desconoce.
    pub bitrate: Option<u32>,
    /// Número de trabajo de la actualización de la base en curso, si hay una.
    pub updating_db: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Song {
    pub file: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    /// Tag libre del usuario (`2003`, `2003-05-12`…); se muestra solo si trae el año.
    pub date: Option<String>,
    pub track: Option<String>,
    pub duration: Option<Duration>,
    /// Solo presente en canciones de la cola.
    pub position: Option<u32>,
}

impl Song {
    /// Título a mostrar: el tag `Title` o, si falta, el nombre del archivo.
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or_else(|| self.file_name())
    }

    /// Artista que agrupa el álbum: `AlbumArtist` o, si falta, `Artist`.
    pub fn album_owner(&self) -> Option<&str> {
        self.album_artist.as_deref().or(self.artist.as_deref())
    }

    /// Nombre del archivo, sin los directorios que lo contienen.
    pub fn file_name(&self) -> &str {
        last_component(&self.file)
    }
}

/// Último tramo de una ruta de la biblioteca.
fn last_component(path: &str) -> &str {
    path.rsplit(PATH_SEPARATOR).next().unwrap_or(path)
}

/// Elemento al navegar la biblioteca por directorios.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Directory { path: String },
    File(Box<Song>),
    Playlist { path: String },
}

impl Entry {
    /// Ruta dentro de la biblioteca, sea directorio, archivo o playlist.
    pub fn path(&self) -> &str {
        match self {
            Entry::Directory { path } | Entry::Playlist { path } => path,
            Entry::File(song) => &song.file,
        }
    }

    /// Nombre sin los directorios que lo contienen.
    pub fn name(&self) -> &str {
        last_component(self.path())
    }
}

/// Áreas del servidor cuyo cambio se notifica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subsystem {
    Database,
    StoredPlaylist,
    Queue,
    Player,
    Mixer,
    Options,
    /// Una actualización de la base empezó o terminó.
    Update,
}

/// Tags por los que se lista, filtra y busca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Artist,
    AlbumArtist,
    Album,
    Title,
    /// Cualquier tag o el nombre del archivo; solo para búsquedas.
    Any,
}

/// Restricción "el tag es exactamente este valor".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFilter {
    pub tag: Tag,
    pub value: String,
}

impl TagFilter {
    pub fn new(tag: Tag, value: impl Into<String>) -> Self {
        Self {
            tag,
            value: value.into(),
        }
    }
}
