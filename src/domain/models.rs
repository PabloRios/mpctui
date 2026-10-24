//! Modelos del dominio compartidos por todas las capas.

use std::time::Duration;

use super::audio::AudioFormat;
use super::inputs::{PATH_SEPARATOR, PartitionName, Volume};

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
    /// Número de disco (`1`, `2/2`…).
    pub disc: Option<String>,
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

    /// Número de pista sin el total del disco (`3/12` → `3`).
    pub fn track_number(&self) -> Option<&str> {
        number_before_total(self.track.as_deref())
    }

    /// Número de disco sin el total de discos (`1/2` → `1`).
    pub fn disc_number(&self) -> Option<&str> {
        number_before_total(self.disc.as_deref())
    }

    /// Un stream (`http://…`) no está en la biblioteca.
    pub fn is_stream(&self) -> bool {
        is_stream_uri(&self.file)
    }

    /// El año, si `Date` empieza con cuatro dígitos; si no, nada (es un tag libre).
    pub fn year(&self) -> Option<&str> {
        let year = self.date.as_deref()?.get(..YEAR_DIGITS)?;
        year.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then_some(year)
    }
}

/// Las URL (`http://…`) son streams: no tienen carpeta ni extensión que diga algo.
const URL_SCHEME_SEPARATOR: &str = "://";

/// Si la URI es un stream y no un archivo de la biblioteca.
pub fn is_stream_uri(uri: &str) -> bool {
    uri.contains(URL_SCHEME_SEPARATOR)
}

/// Separa el número del total en los tags `Track` y `Disc` (`3/12`).
const TOTAL_SEPARATOR: char = '/';
/// Cifras del año al principio de `Date`.
const YEAR_DIGITS: usize = 4;

/// La parte de un tag numerado antes del total.
fn number_before_total(tag: Option<&str>) -> Option<&str> {
    tag?.split(TOTAL_SEPARATOR).next()
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
    /// Una salida de audio se agregó, se quitó, se movió o cambió de estado.
    Outputs,
    /// Una partición se creó, se borró o cambió.
    Partitions,
}

/// Salida de audio de una partición.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Posición en la lista de su partición: es lo que piden `enableoutput` y
    /// `disableoutput`, y puede cambiar entre ejecuciones de MPD.
    pub id: u32,
    pub name: String,
    pub plugin: String,
    pub enabled: bool,
}

/// Partición de MPD con las salidas que le pertenecen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub name: PartitionName,
    pub outputs: Vec<Output>,
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

/// Un `move` de la cola: lleva el tema de la posición `from` a la posición `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongMove {
    pub from: u32,
    pub to: u32,
}

/// Hacia dónde se corre un tema dentro de una lista (cola o playlist).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Up,
    Down,
}

impl MoveDirection {
    /// La posición a la que llega el tema de `position` en una lista de `length`
    /// temas, o `None` si ya está en el borde (o fuera de la lista).
    pub fn target(self, position: u32, length: usize) -> Option<u32> {
        let target = match self {
            MoveDirection::Up => position.checked_sub(1)?,
            MoveDirection::Down => position.checked_add(1)?,
        };
        let in_list = |index: u32| usize::try_from(index).is_ok_and(|index| index < length);
        (in_list(position) && in_list(target)).then_some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_stops_at_the_edges() {
        assert_eq!(MoveDirection::Down.target(0, 3), Some(1));
        assert_eq!(MoveDirection::Down.target(2, 3), None);
        assert_eq!(MoveDirection::Up.target(2, 3), Some(1));
        assert_eq!(MoveDirection::Up.target(0, 3), None);
        assert_eq!(MoveDirection::Down.target(5, 3), None);
        assert_eq!(MoveDirection::Up.target(0, 0), None);
    }

    fn song_with_track(track: &str) -> Song {
        Song {
            track: Some(track.into()),
            ..Song::default()
        }
    }

    #[test]
    fn track_number_drops_the_disc_total() {
        assert_eq!(song_with_track("3/12").track_number(), Some("3"));
        assert_eq!(song_with_track("7").track_number(), Some("7"));
        assert_eq!(Song::default().track_number(), None);
    }

    #[test]
    fn disc_number_drops_the_disc_total() {
        let song = Song {
            disc: Some("2/3".into()),
            ..Song::default()
        };
        assert_eq!(song.disc_number(), Some("2"));
        assert_eq!(Song::default().disc_number(), None);
    }

    #[test]
    fn urls_are_streams() {
        assert!(is_stream_uri("http://radio.example/stream.mp3"));
        assert!(!is_stream_uri("Rock/a.flac"));
        let stream = Song {
            file: "https://radio/".into(),
            ..Song::default()
        };
        assert!(stream.is_stream());
    }

    fn song_with_date(date: &str) -> Song {
        Song {
            date: Some(date.into()),
            ..Song::default()
        }
    }

    #[test]
    fn year_needs_four_leading_digits() {
        assert_eq!(song_with_date("1973-03-01").year(), Some("1973"));
        assert_eq!(song_with_date("1973").year(), Some("1973"));
        assert_eq!(song_with_date("circa 1970").year(), None);
        assert_eq!(song_with_date("97").year(), None);
        assert_eq!(Song::default().year(), None);
    }
}
