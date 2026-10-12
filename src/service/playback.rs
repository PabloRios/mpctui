//! Reglas de reproducción: alternar pausa, pasos de volumen y modos.

use std::time::Duration;

use crate::domain::{PlayState, SingleMode, Song, Status, Volume};
use crate::repository::{OptionsRepository, PlayerRepository, Result};

/// Cada cuánto se vuelve a leer el estado mientras suena: el bitrate cambia sin que
/// `idle` avise, y de paso se corrige la deriva del reloj local.
pub const STATUS_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Estado del reproductor leído en un mismo momento.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerSnapshot {
    pub status: Status,
    pub current_song: Option<Song>,
}

impl PlayerSnapshot {
    /// Solo mientras suena hace falta volver a leer el estado cada `STATUS_POLL_INTERVAL`;
    /// pausado o detenido no cambia nada sin que `idle` avise.
    pub fn needs_polling(&self) -> bool {
        self.status.state == PlayState::Play
    }
}

pub struct PlaybackService<R> {
    repository: R,
}

impl<R: PlayerRepository + OptionsRepository + Send> PlaybackService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn snapshot(&mut self) -> Result<PlayerSnapshot> {
        Ok(PlayerSnapshot {
            status: self.repository.status().await?,
            current_song: self.repository.current_song().await?,
        })
    }

    /// Solo el estado, para las relecturas periódicas (el tema lo avisa `idle`).
    pub async fn status(&mut self) -> Result<Status> {
        self.repository.status().await
    }

    /// Pausa si suena; reanuda si está en pausa; arranca si está detenido.
    pub async fn toggle_pause(&mut self, state: PlayState) -> Result<()> {
        match state {
            PlayState::Play => self.repository.set_pause(true).await,
            PlayState::Pause => self.repository.set_pause(false).await,
            PlayState::Stop => self.repository.play(None).await,
        }
    }

    pub async fn play_position(&mut self, position: u32) -> Result<()> {
        self.repository.play(Some(position)).await
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.repository.stop().await
    }

    pub async fn next(&mut self) -> Result<()> {
        self.repository.next().await
    }

    pub async fn previous(&mut self) -> Result<()> {
        self.repository.previous().await
    }

    pub async fn change_volume(&mut self, current: Volume, delta: i16) -> Result<()> {
        self.repository.set_volume(current.stepped(delta)).await
    }

    pub async fn seek(&mut self, offset_seconds: i32) -> Result<()> {
        self.repository.seek_relative(offset_seconds).await
    }

    /// Deja sonando el tema actual desde `position`: en pausa reanuda y detenido
    /// arranca (MPD no salta con el reproductor detenido, así que primero arranca).
    pub async fn play_from(&mut self, state: PlayState, position: Duration) -> Result<()> {
        match state {
            PlayState::Play => self.repository.seek_to(position).await,
            PlayState::Pause => {
                self.repository.seek_to(position).await?;
                self.repository.set_pause(false).await
            }
            PlayState::Stop => {
                self.repository.play(None).await?;
                self.repository.seek_to(position).await
            }
        }
    }

    pub async fn toggle_random(&mut self, enabled: bool) -> Result<()> {
        self.repository.set_random(!enabled).await
    }

    pub async fn toggle_repeat(&mut self, enabled: bool) -> Result<()> {
        self.repository.set_repeat(!enabled).await
    }

    pub async fn toggle_consume(&mut self, enabled: bool) -> Result<()> {
        self.repository.set_consume(!enabled).await
    }

    pub async fn cycle_single(&mut self, mode: SingleMode) -> Result<()> {
        self.repository.set_single(mode.next()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Repositorio falso que registra las llamadas recibidas.
    #[derive(Default)]
    struct RecordingPlayer {
        calls: Vec<String>,
    }

    impl RecordingPlayer {
        fn record(&mut self, call: String) -> Result<()> {
            self.calls.push(call);
            Ok(())
        }
    }

    impl PlayerRepository for RecordingPlayer {
        async fn status(&mut self) -> Result<Status> {
            Ok(Status::default())
        }
        async fn current_song(&mut self) -> Result<Option<Song>> {
            Ok(None)
        }
        async fn play(&mut self, position: Option<u32>) -> Result<()> {
            self.record(format!("play {position:?}"))
        }
        async fn set_pause(&mut self, paused: bool) -> Result<()> {
            self.record(format!("pause {paused}"))
        }
        async fn stop(&mut self) -> Result<()> {
            self.record("stop".into())
        }
        async fn next(&mut self) -> Result<()> {
            self.record("next".into())
        }
        async fn previous(&mut self) -> Result<()> {
            self.record("previous".into())
        }
        async fn set_volume(&mut self, volume: Volume) -> Result<()> {
            self.record(format!("volume {}", volume.value()))
        }
        async fn seek_relative(&mut self, offset_seconds: i32) -> Result<()> {
            self.record(format!("seek {offset_seconds}"))
        }
        async fn seek_to(&mut self, position: Duration) -> Result<()> {
            self.record(format!("seek to {position:?}"))
        }
    }

    impl OptionsRepository for RecordingPlayer {
        async fn set_random(&mut self, enabled: bool) -> Result<()> {
            self.record(format!("random {enabled}"))
        }
        async fn set_repeat(&mut self, enabled: bool) -> Result<()> {
            self.record(format!("repeat {enabled}"))
        }
        async fn set_single(&mut self, mode: SingleMode) -> Result<()> {
            self.record(format!("single {mode:?}"))
        }
        async fn set_consume(&mut self, enabled: bool) -> Result<()> {
            self.record(format!("consume {enabled}"))
        }
    }

    #[tokio::test]
    async fn toggle_pause_depends_on_state() {
        let mut service = PlaybackService::new(RecordingPlayer::default());
        service.toggle_pause(PlayState::Play).await.unwrap();
        service.toggle_pause(PlayState::Pause).await.unwrap();
        service.toggle_pause(PlayState::Stop).await.unwrap();
        assert_eq!(
            service.repository.calls,
            ["pause true", "pause false", "play None"]
        );
    }

    #[tokio::test]
    async fn playing_from_a_position_always_leaves_it_playing() {
        let position = Duration::from_millis(75_500);
        let mut service = PlaybackService::new(RecordingPlayer::default());
        service.play_from(PlayState::Play, position).await.unwrap();
        assert_eq!(service.repository.calls, ["seek to 75.5s"]);
        let mut service = PlaybackService::new(RecordingPlayer::default());
        service.play_from(PlayState::Pause, position).await.unwrap();
        assert_eq!(service.repository.calls, ["seek to 75.5s", "pause false"]);
        let mut service = PlaybackService::new(RecordingPlayer::default());
        service.play_from(PlayState::Stop, position).await.unwrap();
        assert_eq!(service.repository.calls, ["play None", "seek to 75.5s"]);
    }

    #[test]
    fn only_playback_needs_polling() {
        let snapshot = |state| PlayerSnapshot {
            status: Status {
                state,
                ..Status::default()
            },
            current_song: None,
        };
        assert!(snapshot(PlayState::Play).needs_polling());
        assert!(!snapshot(PlayState::Pause).needs_polling());
        assert!(!snapshot(PlayState::Stop).needs_polling());
    }

    #[tokio::test]
    async fn volume_and_modes_are_derived_from_current_values() {
        let mut service = PlaybackService::new(RecordingPlayer::default());
        service
            .change_volume(Volume::new(98).unwrap(), 5)
            .await
            .unwrap();
        service.toggle_random(true).await.unwrap();
        service.cycle_single(SingleMode::On).await.unwrap();
        assert_eq!(
            service.repository.calls,
            ["volume 100", "random false", "single Oneshot"]
        );
    }
}
