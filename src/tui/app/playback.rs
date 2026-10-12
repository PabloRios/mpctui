//! Controles de reproducción, válidos en cualquier vista.

use super::{App, Backend};
use crate::i18n::Phrase;
use crate::tui::action::Action;

const VOLUME_STEP: i16 = 5;
const SEEK_STEP_SECONDS: i32 = 5;

impl<B: Backend> App<B> {
    pub(super) async fn perform_playback(&mut self, action: Action) {
        let status = self.state.player.status.clone();
        let playback = &mut self.services.playback;
        let result = match action {
            Action::TogglePause => playback.toggle_pause(status.state).await,
            Action::Stop => playback.stop().await,
            Action::NextSong => playback.next().await,
            Action::PreviousSong => playback.previous().await,
            Action::SeekForward => playback.seek(SEEK_STEP_SECONDS).await,
            Action::SeekBackward => playback.seek(-SEEK_STEP_SECONDS).await,
            Action::ToggleRandom => playback.toggle_random(status.random).await,
            Action::ToggleRepeat => playback.toggle_repeat(status.repeat).await,
            Action::CycleSingle => playback.cycle_single(status.single).await,
            Action::ToggleConsume => playback.toggle_consume(status.consume).await,
            Action::VolumeUp => return self.change_volume(VOLUME_STEP).await,
            Action::VolumeDown => return self.change_volume(-VOLUME_STEP).await,
            _ => return,
        };
        self.report(result);
    }

    /// Sin mixer no hay volumen que cambiar: se avisa en vez de llamar al servidor.
    async fn change_volume(&mut self, delta: i16) {
        let Some(volume) = self.state.player.status.volume else {
            return self.notify_error(self.phrase(Phrase::NoVolumeControl));
        };
        let result = self.services.playback.change_volume(volume, delta).await;
        self.report(result);
    }
}
