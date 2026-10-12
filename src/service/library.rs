//! Consultas a la biblioteca: por tags, por directorios y búsqueda.

use super::filters::{album_filters, artist_filters};
use crate::domain::{Entry, SearchText, Song, Tag};
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
}
