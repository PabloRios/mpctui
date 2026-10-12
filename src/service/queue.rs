//! Operaciones sobre la cola de reproducción.

use super::filters::{album_filters, artist_filters};
use crate::domain::{MoveDirection, Song};
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

    /// Corre el tema un lugar. Devuelve su nueva posición, o `None` si ya estaba en el
    /// borde (sin pedir nada al servidor).
    pub async fn shift(
        &mut self,
        position: u32,
        direction: MoveDirection,
        length: usize,
    ) -> Result<Option<u32>> {
        let Some(target) = direction.target(position, length) else {
            return Ok(None);
        };
        self.repository.move_song(position, target).await?;
        Ok(Some(target))
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.repository.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TagFilter;

    /// Registra los movimientos pedidos.
    #[derive(Default)]
    struct RecordingQueue {
        moves: Vec<(u32, u32)>,
    }

    impl QueueRepository for RecordingQueue {
        async fn queue(&mut self) -> Result<Vec<Song>> {
            Ok(Vec::new())
        }

        async fn add(&mut self, _uri: &str) -> Result<()> {
            Ok(())
        }

        async fn add_matching(&mut self, _filters: &[TagFilter]) -> Result<()> {
            Ok(())
        }

        async fn delete(&mut self, _position: u32) -> Result<()> {
            Ok(())
        }

        async fn move_song(&mut self, from: u32, to: u32) -> Result<()> {
            self.moves.push((from, to));
            Ok(())
        }

        async fn clear(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn shift_moves_one_place_and_stops_at_the_edges() {
        let mut service = QueueService::new(RecordingQueue::default());
        assert_eq!(service.shift(1, MoveDirection::Down, 3).await, Ok(Some(2)));
        assert_eq!(service.shift(1, MoveDirection::Up, 3).await, Ok(Some(0)));
        assert_eq!(service.shift(2, MoveDirection::Down, 3).await, Ok(None));
        assert_eq!(service.shift(0, MoveDirection::Up, 3).await, Ok(None));
        assert_eq!(service.repository.moves, [(1, 2), (1, 0)]);
    }
}
