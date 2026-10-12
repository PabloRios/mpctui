//! Playlists guardadas en el servidor.

use crate::domain::{PlaylistName, Song};
use crate::repository::{PlaylistRepository, Result};

pub struct PlaylistService<R> {
    repository: R,
}

impl<R: PlaylistRepository + Send> PlaylistService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn names(&mut self) -> Result<Vec<PlaylistName>> {
        let mut names = self.repository.playlists().await?;
        names.sort();
        Ok(names)
    }

    pub async fn songs(&mut self, name: &PlaylistName) -> Result<Vec<Song>> {
        self.repository.playlist_songs(name).await
    }

    /// Agrega la playlist al final de la cola.
    pub async fn load(&mut self, name: &PlaylistName) -> Result<()> {
        self.repository.load(name).await
    }

    /// Guarda la cola actual con ese nombre.
    pub async fn save_queue(&mut self, name: &PlaylistName) -> Result<()> {
        self.repository.save(name).await
    }

    pub async fn remove(&mut self, name: &PlaylistName) -> Result<()> {
        self.repository.remove(name).await
    }
}
