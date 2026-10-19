//! Criterios para ordenar temas. Se encadenan: el segundo desempata al primero, y así.
//! Un tema sin el dato va antes que los que lo tienen.

use std::cmp::Ordering;

use super::models::Song;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// El artista que agrupa el álbum (`AlbumArtist` o, si falta, `Artist`).
    Artist,
    Year,
    Album,
    Disc,
    Track,
    Title,
    File,
}

impl SortKey {
    /// Todos, en el orden en que se ofrecen.
    pub const ALL: [SortKey; 7] = [
        SortKey::Artist,
        SortKey::Year,
        SortKey::Album,
        SortKey::Disc,
        SortKey::Track,
        SortKey::Title,
        SortKey::File,
    ];

    fn compare(self, first: &Song, second: &Song) -> Ordering {
        match self {
            SortKey::Artist => compare_text(first.album_owner(), second.album_owner()),
            SortKey::Year => first.year().cmp(&second.year()),
            SortKey::Album => compare_text(first.album.as_deref(), second.album.as_deref()),
            SortKey::Disc => compare_number(first.disc_number(), second.disc_number()),
            SortKey::Track => compare_number(first.track_number(), second.track_number()),
            SortKey::Title => compare_text(first.title.as_deref(), second.title.as_deref()),
            SortKey::File => first.file.cmp(&second.file),
        }
    }
}

/// Compara por cada criterio, en orden, hasta que uno desempate.
pub fn compare_songs(keys: &[SortKey], first: &Song, second: &Song) -> Ordering {
    keys.iter().fold(Ordering::Equal, |ordering, key| {
        ordering.then_with(|| key.compare(first, second))
    })
}

/// Sin distinguir mayúsculas.
fn compare_text(first: Option<&str>, second: Option<&str>) -> Ordering {
    let lowercase = |text: Option<&str>| text.map(str::to_lowercase);
    lowercase(first).cmp(&lowercase(second))
}

/// Como número; lo que no es un número cuenta como faltante.
fn compare_number(first: Option<&str>, second: Option<&str>) -> Ordering {
    let number = |text: Option<&str>| text.and_then(|text| text.trim().parse::<u32>().ok());
    number(first).cmp(&number(second))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(file: &str, artist: Option<&str>, date: Option<&str>, album: &str) -> Song {
        Song {
            file: file.into(),
            artist: artist.map(Into::into),
            date: date.map(Into::into),
            album: Some(album.into()),
            ..Song::default()
        }
    }

    fn sorted_files(keys: &[SortKey], mut songs: Vec<Song>) -> Vec<String> {
        songs.sort_by(|first, second| compare_songs(keys, first, second));
        songs.into_iter().map(|song| song.file).collect()
    }

    fn library() -> Vec<Song> {
        vec![
            song("b-2001", Some("Beta"), Some("2001"), "Dos"),
            song("a-2005", Some("alfa"), Some("2005"), "Tres"),
            song("b-1999", Some("Beta"), Some("1999"), "Uno"),
            song("a-1999", Some("Alfa"), Some("1999-05-01"), "Cero"),
        ]
    }

    #[test]
    fn artist_then_year_groups_each_artist() {
        let keys = [SortKey::Artist, SortKey::Year, SortKey::Album];
        assert_eq!(
            sorted_files(&keys, library()),
            ["a-1999", "a-2005", "b-1999", "b-2001"]
        );
    }

    #[test]
    fn year_then_artist_follows_the_calendar() {
        let keys = [SortKey::Year, SortKey::Artist, SortKey::Album];
        assert_eq!(
            sorted_files(&keys, library()),
            ["a-1999", "b-1999", "b-2001", "a-2005"]
        );
    }

    #[test]
    fn songs_without_the_tag_come_first() {
        let songs = vec![
            song("con", Some("Alfa"), Some("1990"), "Uno"),
            song("sin", None, None, "Uno"),
        ];
        assert_eq!(
            sorted_files(&[SortKey::Artist], songs.clone()),
            ["sin", "con"]
        );
        assert_eq!(sorted_files(&[SortKey::Year], songs), ["sin", "con"]);
    }

    fn numbered(file: &str, disc: &str, track: &str) -> Song {
        Song {
            file: file.into(),
            disc: Some(disc.into()),
            track: Some(track.into()),
            ..Song::default()
        }
    }

    #[test]
    fn disc_and_track_compare_as_numbers_without_the_total() {
        let songs = vec![
            numbered("2-1", "2/2", "1/9"),
            numbered("1-10", "1/2", "10/12"),
            numbered("1-2", "1/2", "2/12"),
        ];
        assert_eq!(
            sorted_files(&[SortKey::Disc, SortKey::Track], songs),
            ["1-2", "1-10", "2-1"]
        );
    }

    #[test]
    fn ties_keep_the_original_order() {
        let songs = vec![
            song("primero", Some("Alfa"), None, "Uno"),
            song("segundo", Some("ALFA"), None, "uno"),
        ];
        let keys = [SortKey::Artist, SortKey::Album];
        assert_eq!(sorted_files(&keys, songs), ["primero", "segundo"]);
    }
}
