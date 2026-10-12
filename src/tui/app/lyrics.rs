//! Letra: pide la del tema actual cuando cambia (con la vista Letra activa o, si se
//! configuró, siempre) y redibuja justo cuando empieza la línea siguiente de una letra
//! sincronizada que se ve (en la vista o en el panel "ahora suena").

use std::time::Instant;

use super::{App, Backend};
use crate::domain::LyricsQuery;
use crate::service::{LyricsLookup, LyricsOutcome, LyricsResult};
use crate::tui::action::{Action, Navigation};
use crate::tui::state::{LyricsStatus, ViewKind};

/// Líneas que salta una página en una letra sin sincronizar.
const PAGE_LINES: usize = 10;

impl<B: Backend> App<B> {
    /// Antes de cada dibujado: si hace falta la letra y cambió el tema, pide la del
    /// nuevo. Volver a la vista con el mismo tema no busca de nuevo.
    pub(super) fn sync_lyrics(&mut self) {
        let background = self.services.lyrics_in_background;
        if !needs_lyrics(self.state.view, background, self.appearance.lyrics_line) {
            return;
        }
        let Some(lookup) = &self.services.lyrics else {
            return;
        };
        let query = self
            .state
            .player
            .current_song
            .as_ref()
            .map(LyricsQuery::for_song);
        let view = &mut self.state.lyrics;
        if view.query == query {
            return;
        }
        view.scroll = 0;
        view.status = match &query {
            Some(query) => {
                lookup.request(query.clone());
                LyricsStatus::Searching
            }
            None => LyricsStatus::NoSong,
        };
        view.query = query;
    }

    /// Un resultado de un tema que ya no suena se descarta.
    pub(super) fn apply_lyrics(&mut self, result: LyricsResult) {
        let view = &mut self.state.lyrics;
        if view.query.as_ref() != Some(&result.query) {
            return;
        }
        view.status = match result.outcome {
            LyricsOutcome::Found(lyrics) => LyricsStatus::Found(lyrics),
            LyricsOutcome::Instrumental => LyricsStatus::Instrumental,
            LyricsOutcome::NotFound { problem } => LyricsStatus::NotFound {
                untitled: result.query.track.is_none(),
                problem,
            },
        };
    }

    /// Recorre una letra sin sincronizar; la vista limita el desplazamiento al dibujar.
    pub(super) fn lyrics_action(&mut self, action: Action) {
        let Action::Navigate(navigation) = action else {
            return;
        };
        let scroll = &mut self.state.lyrics.scroll;
        *scroll = match navigation {
            Navigation::Next => scroll.saturating_add(1),
            Navigation::Previous => scroll.saturating_sub(1),
            Navigation::PageDown => scroll.saturating_add(PAGE_LINES),
            Navigation::PageUp => scroll.saturating_sub(PAGE_LINES),
            Navigation::First => 0,
            Navigation::Last => usize::MAX,
        };
    }

    /// Cuándo empieza la próxima línea de la letra sincronizada que se ve, si suena.
    pub(super) fn next_lyrics_line(&self) -> Option<Instant> {
        let visible = shows_current_line(self.state.view, self.appearance.lyrics_line);
        if !visible || !self.state.clock.is_running() {
            return None;
        }
        let LyricsStatus::Found(lyrics) = &self.state.lyrics.status else {
            return None;
        };
        let now = Instant::now();
        let elapsed = self.state.clock.elapsed_at(now);
        lyrics
            .next_change(elapsed)
            .map(|next| now + next.saturating_sub(elapsed))
    }
}

/// La letra hace falta con la vista Letra activa, con la búsqueda en segundo plano o si
/// el panel muestra su línea actual.
fn needs_lyrics(view: ViewKind, background: bool, panel_line: bool) -> bool {
    view == ViewKind::Lyrics || background || panel_line
}

/// La línea actual se ve en la vista Letra o, si se configuró, en el panel.
fn shows_current_line(view: ViewKind, panel_line: bool) -> bool {
    view == ViewKind::Lyrics || panel_line
}

/// El próximo resultado de la búsqueda; con las letras desactivadas, espera para siempre.
pub(super) async fn next_result(lookup: &mut Option<LyricsLookup>) -> Option<LyricsResult> {
    match lookup {
        Some(lookup) => lookup.next_result().await,
        None => std::future::pending().await,
    }
}

/// Espera hasta `deadline`; sin plazo, para siempre.
pub(super) async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline.into()).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyrics_are_needed_outside_their_view_only_if_configured() {
        assert!(needs_lyrics(ViewKind::Lyrics, false, false));
        assert!(!needs_lyrics(ViewKind::Queue, false, false));
        assert!(needs_lyrics(ViewKind::Queue, true, false));
        assert!(needs_lyrics(ViewKind::Queue, false, true));
    }

    #[test]
    fn the_current_line_is_visible_in_the_view_or_the_panel() {
        assert!(shows_current_line(ViewKind::Lyrics, false));
        assert!(!shows_current_line(ViewKind::Library, false));
        assert!(shows_current_line(ViewKind::Library, true));
    }
}
