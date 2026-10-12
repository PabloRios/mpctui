//! Letras: de dónde salen, el tema que se busca y las líneas (con tiempo si vienen
//! sincronizadas en formato LRC).

use std::time::Duration;

use super::inputs::{InputError, validate_single_line};
use super::models::Song;

/// Dónde se buscan las letras; el orden lo elige la configuración.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsSource {
    /// `~/.lyrics/Artista - Título.lrc` (compatible con ncmpcpp y ncmpc).
    Cache,
    /// Un `.lrc`/`.txt` con el mismo nombre que el tema, en su carpeta.
    SongFolder,
    /// La letra embebida en los tags del archivo.
    Tags,
    /// lrclib.net.
    Lrclib,
}

impl LyricsSource {
    pub const ALL: [LyricsSource; 4] = [
        LyricsSource::Cache,
        LyricsSource::SongFolder,
        LyricsSource::Tags,
        LyricsSource::Lrclib,
    ];
}

/// Artista y título de un tema, aptos para nombres de archivo y consultas: no vacíos y
/// de una sola línea. El álbum y la duración ayudan a elegir la versión correcta.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackIdentity {
    artist: String,
    title: String,
    album: Option<String>,
    duration: Option<Duration>,
}

impl TrackIdentity {
    pub fn new(
        artist: &str,
        title: &str,
        album: Option<&str>,
        duration: Option<Duration>,
    ) -> Result<Self, InputError> {
        Ok(Self {
            artist: validate_single_line(artist)?.to_owned(),
            title: validate_single_line(title)?.to_owned(),
            album: album
                .map(validate_single_line)
                .transpose()
                .ok()
                .flatten()
                .map(str::to_owned),
            duration,
        })
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn album(&self) -> Option<&str> {
        self.album.as_deref()
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }
}

/// Lo que se busca: la ruta del tema (carpeta y tags) y, si los tags alcanzan, su
/// identidad (caché y LRCLIB). Se usa el tag `Title`, nunca el nombre del archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsQuery {
    pub uri: String,
    pub track: Option<TrackIdentity>,
}

impl LyricsQuery {
    pub fn for_song(song: &Song) -> Self {
        let artist = song.artist.as_deref().or(song.album_artist.as_deref());
        let track = match (artist, song.title.as_deref()) {
            (Some(artist), Some(title)) => {
                TrackIdentity::new(artist, title, song.album.as_deref(), song.duration).ok()
            }
            _ => None,
        };
        Self {
            uri: song.file.clone(),
            track,
        }
    }
}

/// Lo que devuelve una fuente remota de letras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchedLyrics {
    /// LRC o texto plano.
    Text(String),
    /// El tema no tiene letra.
    Instrumental,
    Missing,
}

/// Una línea de la letra; con tiempo si es sincronizada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    pub time: Option<Duration>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lyrics {
    /// Las sincronizadas vienen ordenadas por tiempo.
    pub lines: Vec<LyricLine>,
    pub source: LyricsSource,
}

impl Lyrics {
    /// Todas las líneas tienen tiempo.
    pub fn is_synced(&self) -> bool {
        !self.lines.is_empty() && self.lines.iter().all(|line| line.time.is_some())
    }

    /// La última línea que ya empezó en `elapsed`; `None` antes de la primera o si no
    /// es sincronizada.
    pub fn current_line(&self, elapsed: Duration) -> Option<usize> {
        if !self.is_synced() {
            return None;
        }
        self.lines
            .partition_point(|line| line.time.is_some_and(|time| time <= elapsed))
            .checked_sub(1)
    }

    /// Cuándo empieza la línea siguiente a `elapsed`, si hay.
    pub fn next_change(&self, elapsed: Duration) -> Option<Duration> {
        if !self.is_synced() {
            return None;
        }
        self.lines
            .iter()
            .filter_map(|line| line.time)
            .find(|time| *time > elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(seconds: u64, text: &str) -> LyricLine {
        LyricLine {
            time: Some(Duration::from_secs(seconds)),
            text: text.into(),
        }
    }

    fn synced() -> Lyrics {
        Lyrics {
            lines: vec![line(5, "uno"), line(10, "dos"), line(20, "tres")],
            source: LyricsSource::Cache,
        }
    }

    #[test]
    fn the_current_line_is_the_last_one_that_started() {
        let lyrics = synced();
        assert_eq!(lyrics.current_line(Duration::from_secs(1)), None);
        assert_eq!(lyrics.current_line(Duration::from_secs(10)), Some(1));
        assert_eq!(lyrics.current_line(Duration::from_millis(19_999)), Some(1));
        assert_eq!(lyrics.current_line(Duration::from_secs(300)), Some(2));
    }

    #[test]
    fn the_next_change_is_the_next_line_start() {
        let lyrics = synced();
        assert_eq!(
            lyrics.next_change(Duration::ZERO),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            lyrics.next_change(Duration::from_secs(10)),
            Some(Duration::from_secs(20))
        );
        assert_eq!(lyrics.next_change(Duration::from_secs(20)), None);
    }

    #[test]
    fn plain_lyrics_have_no_current_line() {
        let plain = Lyrics {
            lines: vec![LyricLine {
                time: None,
                text: "sin tiempo".into(),
            }],
            source: LyricsSource::Tags,
        };
        assert!(!plain.is_synced());
        assert_eq!(plain.current_line(Duration::from_secs(5)), None);
        assert_eq!(plain.next_change(Duration::ZERO), None);
    }

    #[test]
    fn a_query_needs_artist_and_title_tags() {
        let song = Song {
            file: "a/b.flac".into(),
            album_artist: Some("Pink Floyd".into()),
            title: Some("Time".into()),
            album: Some("\n".into()),
            ..Song::default()
        };
        let query = LyricsQuery::for_song(&song);
        let track = query.track.unwrap();
        assert_eq!((track.artist(), track.title()), ("Pink Floyd", "Time"));
        assert_eq!(track.album(), None);
        let untitled = Song {
            title: None,
            ..song
        };
        assert_eq!(LyricsQuery::for_song(&untitled).track, None);
        assert!(TrackIdentity::new("a\nb", "c", None, None).is_err());
    }
}
