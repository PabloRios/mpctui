//! Operaciones sobre la cola de reproducción.

use super::filters::{album_filters, artist_filters};
use crate::domain::Song;
use crate::repository::{QueueRepository, Result};

pub struct QueueService<R> {
    repository: R,
}

impl<R: QueueRepository + Send> QueueService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn songs(&mut self) -> Result<Vec<Song>> {
        self.repository.queue().await
    }

    /// Agrega un archivo o un directorio completo.
    pub async fn add_uri(&mut self, uri: &str) -> Result<()> {
        self.repository.add(uri).await
    }

    pub async fn add_songs(&mut self, songs: &[Song]) -> Result<()> {
        for song in songs {
            self.repository.add(&song.file).await?;
        }
        Ok(())
    }

    pub async fn add_artist(&mut self, artist: &str) -> Result<()> {
        self.repository.add_matching(&artist_filters(artist)).await
    }

    pub async fn add_album(&mut self, artist: &str, album: &str) -> Result<()> {
        self.repository
            .add_matching(&album_filters(artist, album))
            .await
    }

    pub async fn delete(&mut self, position: u32) -> Result<()> {
        self.repository.delete(position).await
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.repository.clear().await
    }
}
