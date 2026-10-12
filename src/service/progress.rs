//! Interpolación local del progreso, para no consultar a MPD en cada tick.

use std::time::{Duration, Instant};

use crate::domain::{PlayState, Status};

/// Posición conocida del tema en un instante; mientras suena, avanza con el reloj.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackClock {
    elapsed: Duration,
    duration: Option<Duration>,
    playing: bool,
    synced_at: Instant,
}

impl PlaybackClock {
    pub fn new(status: &Status, synced_at: Instant) -> Self {
        Self {
            elapsed: status.elapsed.unwrap_or_default(),
            duration: status.duration,
            playing: status.state == PlayState::Play,
            synced_at,
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn elapsed_at(&self, now: Instant) -> Duration {
        let elapsed = if self.playing {
            self.elapsed + now.saturating_duration_since(self.synced_at)
        } else {
            self.elapsed
        };
        match self.duration {
            Some(duration) => elapsed.min(duration),
            None => elapsed,
        }
    }

    /// Fracción reproducida entre 0 y 1; 0 si no se conoce la duración.
    pub fn ratio_at(&self, now: Instant) -> f64 {
        match self.duration {
            Some(duration) if !duration.is_zero() => {
                self.elapsed_at(now).as_secs_f64() / duration.as_secs_f64()
            }
            _ => 0.0,
        }
    }
}

impl Default for PlaybackClock {
    fn default() -> Self {
        Self::new(&Status::default(), Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(state: PlayState, elapsed_seconds: u64, duration_seconds: u64) -> Status {
        Status {
            state,
            elapsed: Some(Duration::from_secs(elapsed_seconds)),
            duration: Some(Duration::from_secs(duration_seconds)),
            ..Status::default()
        }
    }

    #[test]
    fn advances_only_while_playing() {
        let start = Instant::now();
        let later = start + Duration::from_secs(10);
        let playing = PlaybackClock::new(&status(PlayState::Play, 30, 200), start);
        let paused = PlaybackClock::new(&status(PlayState::Pause, 30, 200), start);
        assert_eq!(playing.elapsed_at(later), Duration::from_secs(40));
        assert_eq!(paused.elapsed_at(later), Duration::from_secs(30));
        assert_eq!(playing.ratio_at(later), 0.2);
    }

    #[test]
    fn never_passes_song_duration() {
        let start = Instant::now();
        let clock = PlaybackClock::new(&status(PlayState::Play, 195, 200), start);
        let later = start + Duration::from_secs(60);
        assert_eq!(clock.elapsed_at(later), Duration::from_secs(200));
        assert_eq!(clock.ratio_at(later), 1.0);
    }

    #[test]
    fn unknown_duration_has_zero_ratio() {
        let clock = PlaybackClock::default();
        assert_eq!(clock.duration(), None);
        assert_eq!(clock.ratio_at(Instant::now()), 0.0);
    }
}
