//! Ficha de un tema: todos sus tags y los datos del archivo.

use std::cmp::Ordering;
use std::time::Duration;

use super::audio::AudioFormat;
use super::models::Song;

/// Nombre de un tag. Los conocidos se declaran en el orden en que se muestran; el
/// resto se guarda con el nombre que le da el servidor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TagName {
    Title,
    Artist,
    Album,
    AlbumArtist,
    Date,
    OriginalDate,
    Track,
    Disc,
    Genre,
    Composer,
    Performer,
    Conductor,
    Comment,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongTag {
    pub name: TagName,
    pub value: String,
}

impl SongTag {
    pub fn new(name: TagName, value: impl Into<String>) -> Self {
        Self {
            name,
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SongDetails {
    pub file: String,
    pub duration: Option<Duration>,
    /// Formato del archivo (no el de la salida).
    pub format: Option<AudioFormat>,
    /// Fecha de modificación tal como la informa el servidor (`2019-04-21T19:23:18Z`).
    pub last_modified: Option<String>,
    /// En el orden del servidor; un tag con varios valores aparece varias veces.
    pub tags: Vec<SongTag>,
}

impl SongDetails {
    /// La ficha con lo que ya se conoce del tema, para cuando el servidor no la tiene
    /// (streams o archivos que ya no están en la base).
    pub fn from_song(song: &Song) -> Self {
        let known = [
            (TagName::Title, &song.title),
            (TagName::Artist, &song.artist),
            (TagName::Album, &song.album),
            (TagName::AlbumArtist, &song.album_artist),
            (TagName::Date, &song.date),
            (TagName::Track, &song.track),
            (TagName::Disc, &song.disc),
        ];
        Self {
            file: song.file.clone(),
            duration: song.duration,
            format: None,
            last_modified: None,
            tags: known
                .into_iter()
                .filter_map(|(name, value)| Some(SongTag::new(name, value.as_deref()?)))
                .collect(),
        }
    }

    /// Los tags conocidos primero, en su orden, y después los demás en el del servidor.
    pub fn ordered_tags(&self) -> Vec<&SongTag> {
        let mut tags: Vec<&SongTag> = self.tags.iter().collect();
        // Entre los que no se conocen no hay orden: el estable deja el del servidor.
        tags.sort_by(|left, right| match (&left.name, &right.name) {
            (TagName::Other(_), TagName::Other(_)) => Ordering::Equal,
            (left, right) => left.cmp(right),
        });
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_song_keeps_the_known_fields() {
        let song = Song {
            file: "http://radio/".into(),
            title: Some("Noticias".into()),
            track: Some("3".into()),
            duration: Some(Duration::from_secs(60)),
            ..Song::default()
        };
        let details = SongDetails::from_song(&song);
        assert_eq!(details.file, "http://radio/");
        assert_eq!(details.duration, Some(Duration::from_secs(60)));
        assert_eq!(
            details.tags,
            [
                SongTag::new(TagName::Title, "Noticias"),
                SongTag::new(TagName::Track, "3"),
            ]
        );
    }

    #[test]
    fn known_tags_come_first_and_the_rest_keep_their_order() {
        let details = SongDetails {
            tags: vec![
                SongTag::new(TagName::Other("Ensemble".into()), "E"),
                SongTag::new(TagName::Genre, "Rock"),
                SongTag::new(TagName::Artist, "A"),
                SongTag::new(TagName::Other("Label".into()), "L"),
                SongTag::new(TagName::Artist, "B"),
                SongTag::new(TagName::Title, "T"),
            ],
            ..SongDetails::default()
        };
        let values: Vec<_> = details
            .ordered_tags()
            .into_iter()
            .map(|tag| tag.value.as_str())
            .collect();
        assert_eq!(values, ["T", "A", "B", "Rock", "E", "L"]);
    }
}
