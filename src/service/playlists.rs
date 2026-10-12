//! Playlists guardadas en el servidor.

use crate::domain::{MoveDirection, PlaylistName, Song};
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

    /// Agrega archivos o directorios al final de la playlist; si no existe, la crea.
    pub async fn add_uris(&mut self, name: &PlaylistName, uris: &[String]) -> Result<()> {
        for uri in uris {
            self.repository.add_to_playlist(name, uri).await?;
        }
        Ok(())
    }

    pub async fn delete_song(&mut self, name: &PlaylistName, position: u32) -> Result<()> {
        self.repository.delete_from_playlist(name, position).await
    }

    /// Corre el tema un lugar dentro de la playlist. Devuelve su nueva posición, o
    /// `None` si ya estaba en el borde (sin pedir nada al servidor).
    pub async fn shift(
        &mut self,
        name: &PlaylistName,
        position: u32,
        direction: MoveDirection,
        length: usize,
    ) -> Result<Option<u32>> {
        let Some(target) = direction.target(position, length) else {
            return Ok(None);
        };
        self.repository
            .move_in_playlist(name, position, target)
            .await?;
        Ok(Some(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Registra las URI agregadas y los movimientos pedidos.
    #[derive(Default)]
    struct RecordingPlaylists {
        added: Vec<String>,
        moves: Vec<(u32, u32)>,
    }

    impl PlaylistRepository for RecordingPlaylists {
        async fn playlists(&mut self) -> Result<Vec<PlaylistName>> {
            Ok(Vec::new())
        }

        async fn playlist_songs(&mut self, _name: &PlaylistName) -> Result<Vec<Song>> {
            Ok(Vec::new())
        }

        async fn load(&mut self, _name: &PlaylistName) -> Result<()> {
            Ok(())
        }

        async fn save(&mut self, _name: &PlaylistName) -> Result<()> {
            Ok(())
        }

        async fn remove(&mut self, _name: &PlaylistName) -> Result<()> {
            Ok(())
        }

        async fn add_to_playlist(&mut self, _name: &PlaylistName, uri: &str) -> Result<()> {
            self.added.push(uri.to_owned());
            Ok(())
        }

        async fn delete_from_playlist(
            &mut self,
            _name: &PlaylistName,
            _position: u32,
        ) -> Result<()> {
            Ok(())
        }

        async fn move_in_playlist(
            &mut self,
            _name: &PlaylistName,
            from: u32,
            to: u32,
        ) -> Result<()> {
            self.moves.push((from, to));
            Ok(())
        }
    }

    fn rock() -> PlaylistName {
        PlaylistName::new("Rock").unwrap()
    }

    #[tokio::test]
    async fn shift_moves_one_place_and_stops_at_the_edges() {
        let mut service = PlaylistService::new(RecordingPlaylists::default());
        assert_eq!(
            service.shift(&rock(), 0, MoveDirection::Down, 2).await,
            Ok(Some(1))
        );
        assert_eq!(
            service.shift(&rock(), 1, MoveDirection::Down, 2).await,
            Ok(None)
        );
        assert_eq!(service.repository.moves, [(0, 1)]);
    }

    #[tokio::test]
    async fn adds_every_uri_in_order() {
        let mut service = PlaylistService::new(RecordingPlaylists::default());
        let uris = vec!["a.flac".to_owned(), "discos/b".to_owned()];
        service.add_uris(&rock(), &uris).await.unwrap();
        assert_eq!(service.repository.added, uris);
    }
}
