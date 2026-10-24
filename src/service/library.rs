//! Consultas a la biblioteca: por tags, por directorios y búsqueda.

use super::filters::{album_filters, artist_filters};
use crate::domain::{Entry, SearchText, Song, SongDetails, Tag};
use crate::repository::{LibraryRepository, Result};

/// Raíz de la biblioteca para la navegación por directorios.
pub const LIBRARY_ROOT: &str = "";

pub struct LibraryService<R> {
    repository: R,
}

impl<R: LibraryRepository + Send> LibraryService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn artists(&mut self) -> Result<Vec<String>> {
        self.repository.list_tag(Tag::Artist, &[]).await
    }

    pub async fn albums(&mut self, artist: &str) -> Result<Vec<String>> {
        self.repository
            .list_tag(Tag::Album, &artist_filters(artist))
            .await
    }

    pub async fn artist_songs(&mut self, artist: &str) -> Result<Vec<Song>> {
        self.repository.find(&artist_filters(artist)).await
    }

    pub async fn album_songs(&mut self, artist: &str, album: &str) -> Result<Vec<Song>> {
        self.repository.find(&album_filters(artist, album)).await
    }

    pub async fn browse(&mut self, path: &str) -> Result<Vec<Entry>> {
        self.repository.browse(path).await
    }

    pub async fn search(&mut self, field: Tag, text: &SearchText) -> Result<Vec<Song>> {
        self.repository.search(field, text).await
    }

    pub async fn update_database(&mut self) -> Result<()> {
        self.repository.update_database().await
    }

    /// La ficha del servidor o, si no la tiene, lo que ya se sabe del tema. A un stream
    /// no se le pide: MPD lo abriría para leer los tags y podría tardar hasta el timeout.
    pub async fn song_details(&mut self, song: &Song) -> Result<SongDetails> {
        if song.is_stream() {
            return Ok(SongDetails::from_song(song));
        }
        let details = self.repository.song_details(&song.file).await?;
        Ok(details.unwrap_or_else(|| SongDetails::from_song(song)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::domain::{SongTag, TagFilter, TagName};

    /// Fichas en memoria por URI; anota cuáles se pidieron.
    #[derive(Default)]
    struct Memory {
        details: HashMap<String, SongDetails>,
        requested: Vec<String>,
    }

    impl LibraryRepository for Memory {
        async fn browse(&mut self, _path: &str) -> Result<Vec<Entry>> {
            Ok(Vec::new())
        }

        async fn list_tag(&mut self, _tag: Tag, _filters: &[TagFilter]) -> Result<Vec<String>> {
            Ok(Vec::new())
        }

        async fn find(&mut self, _filters: &[TagFilter]) -> Result<Vec<Song>> {
            Ok(Vec::new())
        }

        async fn search(&mut self, _field: Tag, _text: &SearchText) -> Result<Vec<Song>> {
            Ok(Vec::new())
        }

        async fn song_details(&mut self, uri: &str) -> Result<Option<SongDetails>> {
            self.requested.push(uri.to_owned());
            Ok(self.details.get(uri).cloned())
        }

        async fn update_database(&mut self) -> Result<()> {
            Ok(())
        }
    }

    fn song(file: &str) -> Song {
        Song {
            file: file.into(),
            title: Some("Tema".into()),
            ..Song::default()
        }
    }

    #[tokio::test]
    async fn details_come_from_the_server() {
        let full = SongDetails {
            file: "Rock/a.flac".into(),
            tags: vec![SongTag::new(TagName::Genre, "Rock")],
            ..SongDetails::default()
        };
        let repository = Memory {
            details: HashMap::from([("Rock/a.flac".to_owned(), full.clone())]),
            ..Memory::default()
        };
        let mut service = LibraryService::new(repository);
        assert_eq!(
            service.song_details(&song("Rock/a.flac")).await.unwrap(),
            full
        );
    }

    #[tokio::test]
    async fn a_missing_file_shows_what_is_known() {
        let mut service = LibraryService::new(Memory::default());
        let details = service
            .song_details(&song("Rock/borrado.flac"))
            .await
            .unwrap();
        assert_eq!(details, SongDetails::from_song(&song("Rock/borrado.flac")));
    }

    #[tokio::test]
    async fn streams_are_not_queried() {
        let mut service = LibraryService::new(Memory::default());
        let stream = song("http://radio/");
        let details = service.song_details(&stream).await.unwrap();
        assert_eq!(details, SongDetails::from_song(&stream));
        assert!(service.repository.requested.is_empty());
    }
}
