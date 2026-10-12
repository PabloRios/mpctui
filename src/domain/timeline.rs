//! Línea de tiempo de una lista de temas: tramos consecutivos del mismo álbum y el
//! momento en que empieza cada tema. Sirve para la cola, la búsqueda y las playlists.

use std::ops::Range;
use std::time::Duration;

use super::models::Song;

/// Secuencia consecutiva de temas del mismo álbum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlbumRun {
    /// Índice del primer tema del tramo en la lista.
    pub first_index: usize,
    pub song_count: usize,
    /// `None` si algún tema del tramo no tiene duración.
    pub duration: Option<Duration>,
}

impl AlbumRun {
    pub fn indices(&self) -> Range<usize> {
        self.first_index..self.first_index + self.song_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    pub runs: Vec<AlbumRun>,
    /// Inicio de cada tema desde el principio; `None` si antes hay uno sin duración.
    pub song_offsets: Vec<Option<Duration>>,
    pub total: Option<Duration>,
}

impl Default for Timeline {
    fn default() -> Self {
        timeline(&[])
    }
}

impl Timeline {
    /// Índice del tramo que contiene al tema, en O(log n).
    pub fn run_of(&self, song_index: usize) -> Option<usize> {
        if song_index >= self.song_offsets.len() {
            return None;
        }
        let following = self
            .runs
            .partition_point(|run| run.first_index <= song_index);
        following.checked_sub(1)
    }
}

/// Clave que agrupa temas: álbum y artista del álbum. Los temas sin álbum van juntos.
fn run_key(song: &Song) -> (Option<&str>, Option<&str>) {
    match song.album.as_deref() {
        Some(album) => (Some(album), song.album_owner()),
        None => (None, None),
    }
}

/// Suma una duración que puede ser desconocida; desconocido contagia.
fn add_known(total: Option<Duration>, duration: Option<Duration>) -> Option<Duration> {
    Some(total? + duration?)
}

/// Recorre la lista una vez (O(n)).
pub fn timeline(songs: &[Song]) -> Timeline {
    let mut runs: Vec<AlbumRun> = Vec::new();
    let mut song_offsets = Vec::with_capacity(songs.len());
    let mut elapsed = Some(Duration::ZERO);
    for (index, song) in songs.iter().enumerate() {
        song_offsets.push(elapsed);
        elapsed = add_known(elapsed, song.duration);
        match runs.last_mut() {
            Some(run) if run_key(&songs[run.first_index]) == run_key(song) => {
                run.song_count += 1;
                run.duration = add_known(run.duration, song.duration);
            }
            _ => runs.push(AlbumRun {
                first_index: index,
                song_count: 1,
                duration: song.duration,
            }),
        }
    }
    Timeline {
        runs,
        song_offsets,
        total: elapsed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(album: Option<&str>, artist: &str, seconds: Option<u64>) -> Song {
        Song {
            file: format!("{artist}.mp3"),
            artist: Some(artist.into()),
            album: album.map(str::to_owned),
            duration: seconds.map(Duration::from_secs),
            ..Song::default()
        }
    }

    fn counts(timeline: &Timeline) -> Vec<(usize, usize)> {
        timeline
            .runs
            .iter()
            .map(|run| (run.first_index, run.song_count))
            .collect()
    }

    #[test]
    fn consecutive_songs_of_an_album_form_one_run() {
        let songs = [
            song(Some("Meddle"), "Pink Floyd", Some(60)),
            song(Some("Meddle"), "Pink Floyd", Some(30)),
            song(Some("Animals"), "Pink Floyd", Some(10)),
        ];
        let timeline = timeline(&songs);
        assert_eq!(counts(&timeline), [(0, 2), (2, 1)]);
        assert_eq!(timeline.runs[0].duration, Some(Duration::from_secs(90)));
        assert_eq!(timeline.song_offsets[2], Some(Duration::from_secs(90)));
        assert_eq!(timeline.total, Some(Duration::from_secs(100)));
    }

    #[test]
    fn repeated_album_later_starts_a_new_run() {
        let songs = [
            song(Some("Meddle"), "Pink Floyd", Some(1)),
            song(Some("Animals"), "Pink Floyd", Some(1)),
            song(Some("Meddle"), "Pink Floyd", Some(1)),
        ];
        assert_eq!(counts(&timeline(&songs)), [(0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn compilations_group_by_album_artist() {
        let mut first = song(Some("Éxitos"), "Uno", Some(1));
        let mut second = song(Some("Éxitos"), "Otro", Some(1));
        first.album_artist = Some("Varios".into());
        second.album_artist = Some("Varios".into());
        let without_album_artist = song(Some("Éxitos"), "Tercero", Some(1));
        let timeline = timeline(&[first, second, without_album_artist]);
        assert_eq!(counts(&timeline), [(0, 2), (2, 1)]);
    }

    #[test]
    fn songs_without_album_share_a_run() {
        let songs = [
            song(None, "Uno", Some(1)),
            song(None, "Otro", Some(1)),
            song(Some("Meddle"), "Pink Floyd", Some(1)),
        ];
        assert_eq!(counts(&timeline(&songs)), [(0, 2), (2, 1)]);
    }

    #[test]
    fn unknown_duration_spreads_forward() {
        let songs = [
            song(Some("A"), "X", Some(5)),
            song(Some("A"), "X", None),
            song(Some("B"), "X", Some(5)),
        ];
        let timeline = timeline(&songs);
        assert_eq!(timeline.runs[0].duration, None);
        assert_eq!(timeline.runs[1].duration, Some(Duration::from_secs(5)));
        assert_eq!(timeline.song_offsets[1], Some(Duration::from_secs(5)));
        assert_eq!(timeline.song_offsets[2], None);
        assert_eq!(timeline.total, None);
    }

    #[test]
    fn run_of_finds_the_run_of_each_song() {
        let songs = [
            song(Some("A"), "X", Some(1)),
            song(Some("A"), "X", Some(1)),
            song(Some("B"), "X", Some(1)),
        ];
        let timeline = timeline(&songs);
        assert_eq!(timeline.run_of(1), Some(0));
        assert_eq!(timeline.run_of(2), Some(1));
        assert_eq!(timeline.run_of(3), None);
        assert_eq!(Timeline::default().run_of(0), None);
    }
}
