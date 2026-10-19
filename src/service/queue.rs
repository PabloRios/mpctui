//! Operaciones sobre la cola de reproducción.

use super::filters::{album_filters, artist_filters};
use crate::domain::{MoveDirection, Song, SongMove, SortKey, compare_songs};
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

    /// Reordena la cola según los criterios, en orden. Relee la cola del servidor para
    /// no ordenar una copia vieja; si ya está ordenada, no pide nada más.
    pub async fn sort(&mut self, keys: &[SortKey]) -> Result<()> {
        let songs = self.repository.queue().await?;
        let moves = moves_toward(&sorted_order(&songs, keys));
        if moves.is_empty() {
            return Ok(());
        }
        self.repository.move_songs(&moves).await
    }

    pub async fn clear(&mut self) -> Result<()> {
        self.repository.clear().await
    }
}

/// Las posiciones actuales en el orden pedido. Estable: los empates quedan como están.
fn sorted_order(songs: &[Song], keys: &[SortKey]) -> Vec<u32> {
    let mut positioned: Vec<(u32, &Song)> = (0..).zip(songs).collect();
    positioned.sort_by(|(_, first), (_, second)| compare_songs(keys, first, second));
    positioned
        .into_iter()
        .map(|(position, _)| position)
        .collect()
}

/// Los `move` que llevan la cola al orden pedido (`order[i]` es la posición actual del
/// tema que tiene que quedar en `i`): cada lugar, de arriba abajo, trae a su tema si no
/// está ya ahí. Como mucho, uno menos que la cantidad de temas.
fn moves_toward(order: &[u32]) -> Vec<SongMove> {
    let mut current: Vec<u32> = (0..).take(order.len()).collect();
    let mut moves = Vec::new();
    for ((index, &wanted), to) in order.iter().enumerate().zip(0..) {
        let Some(found) = current[index..].iter().position(|&song| song == wanted) else {
            continue;
        };
        let Ok(distance) = u32::try_from(found) else {
            continue;
        };
        if distance == 0 {
            continue;
        }
        current.remove(index + found);
        current.insert(index, wanted);
        moves.push(SongMove {
            from: to + distance,
            to,
        });
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TagFilter;

    /// Devuelve la cola dada y registra los movimientos pedidos.
    #[derive(Default)]
    struct RecordingQueue {
        songs: Vec<Song>,
        moves: Vec<(u32, u32)>,
        move_lists: Vec<Vec<SongMove>>,
    }

    impl QueueRepository for RecordingQueue {
        async fn queue(&mut self) -> Result<Vec<Song>> {
            Ok(self.songs.clone())
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

        async fn move_songs(&mut self, moves: &[SongMove]) -> Result<()> {
            self.move_lists.push(moves.to_vec());
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

    /// Aplica los movimientos a la cola `0..length` y devuelve qué tema quedó en cada lugar.
    fn apply(moves: &[SongMove], length: u32) -> Vec<u32> {
        let mut queue: Vec<u32> = (0..length).collect();
        for step in moves {
            let song = queue.remove(step.from as usize);
            queue.insert(step.to as usize, song);
        }
        queue
    }

    #[test]
    fn moves_toward_reaches_the_order_with_few_moves() {
        let order = [3, 0, 4, 1, 2];
        let moves = moves_toward(&order);
        assert_eq!(apply(&moves, 5), order);
        assert!(moves.len() < order.len());
    }

    #[test]
    fn an_ordered_queue_needs_no_moves() {
        assert!(moves_toward(&[0, 1, 2]).is_empty());
        assert!(moves_toward(&[]).is_empty());
    }

    fn titled(title: &str) -> Song {
        Song {
            file: format!("{title}.flac"),
            title: Some(title.into()),
            ..Song::default()
        }
    }

    #[tokio::test]
    async fn sort_sends_the_moves_in_one_go() {
        let songs = vec![titled("c"), titled("a"), titled("b")];
        let mut service = QueueService::new(RecordingQueue {
            songs,
            ..RecordingQueue::default()
        });
        assert_eq!(service.sort(&[SortKey::Title]).await, Ok(()));
        assert_eq!(
            service.repository.move_lists,
            [vec![
                SongMove { from: 1, to: 0 },
                SongMove { from: 2, to: 1 }
            ]]
        );
    }

    #[tokio::test]
    async fn sorting_an_ordered_queue_sends_nothing() {
        let songs = vec![titled("a"), titled("b")];
        let mut service = QueueService::new(RecordingQueue {
            songs,
            ..RecordingQueue::default()
        });
        assert_eq!(service.sort(&[SortKey::Title]).await, Ok(()));
        assert!(service.repository.move_lists.is_empty());
    }
}
