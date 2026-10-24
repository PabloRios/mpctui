//! Letra: pide la del tema actual cuando cambia (con la vista Letra activa o, si se
//! configuró, siempre) y redibuja justo cuando empieza la línea siguiente de una letra
//! sincronizada que se ve (en la vista o en el panel "ahora suena"). En una letra
//! sincronizada se elige una línea y `Enter` hace sonar el tema desde ahí.

use std::time::{Duration, Instant};

use super::{App, Backend, PAGE_LINES};
use crate::domain::LyricsQuery;
use crate::service::{LyricsLookup, LyricsOutcome, LyricsResult};
use crate::tui::action::{Action, Navigation};
use crate::tui::state::{LyricsStatus, ViewKind};

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
        view.cursor = None;
        view.status = match &query {
            Some(query) => {
                lookup.request(query.clone());
                LyricsStatus::Searching
            }
            None => LyricsStatus::NoSong,
        };
        view.query = query;
    }

    /// En la vista Letra, si la búsqueda falló, pide de nuevo la letra del mismo tema.
    /// Los errores no se recuerdan, así que se vuelve a probar cada fuente.
    pub(super) fn retry_lyrics(&mut self) {
        let Some(lookup) = &self.services.lyrics else {
            return;
        };
        let view = &mut self.state.lyrics;
        if self.state.view != ViewKind::Lyrics || !view.status.can_retry() {
            return;
        }
        if let Some(query) = &view.query {
            lookup.request(query.clone());
            view.status = LyricsStatus::Searching;
        }
    }

    /// Un resultado de un tema que ya no suena se descarta.
    pub(super) fn apply_lyrics(&mut self, result: LyricsResult) {
        let view = &mut self.state.lyrics;
        if view.query.as_ref() != Some(&result.query) {
            return;
        }
        view.cursor = None;
        view.status = match result.outcome {
            LyricsOutcome::Found(lyrics) => LyricsStatus::Found(lyrics),
            LyricsOutcome::Instrumental => LyricsStatus::Instrumental,
            LyricsOutcome::NotFound { problem } => LyricsStatus::NotFound {
                untitled: result.query.track.is_none(),
                problem,
            },
        };
    }

    /// Letra sincronizada: las teclas de navegación eligen una línea, `Enter` hace sonar
    /// el tema desde ahí y `Back` vuelve a seguir la canción. Sin sincronizar, las
    /// teclas recorren el texto.
    pub(super) async fn lyrics_action(&mut self, action: Action) {
        let LyricsStatus::Found(lyrics) = &self.state.lyrics.status else {
            return;
        };
        if !lyrics.is_synced() {
            return self.scroll_lyrics(action);
        }
        match action {
            Action::Navigate(navigation) => {
                let current = lyrics.current_line(self.state.clock.elapsed_at(Instant::now()));
                let cursor = self.state.lyrics.cursor;
                let target = move_cursor(cursor, current, navigation, lyrics.lines.len());
                self.state.lyrics.cursor = Some(target);
            }
            Action::Activate => {
                let start = self
                    .state
                    .lyrics
                    .cursor
                    .and_then(|index| lyrics.lines.get(index))
                    .and_then(|line| line.time);
                if let Some(start) = start {
                    self.play_from(start).await;
                }
            }
            Action::Back => self.state.lyrics.cursor = None,
            _ => {}
        }
    }

    /// Si salió bien, la vista vuelve a seguir la canción; si no, el cursor queda.
    async fn play_from(&mut self, start: Duration) {
        let state = self.state.player.status.state;
        let result = self.services.playback.play_from(state, start).await;
        if self.report(result).is_some() {
            self.state.lyrics.cursor = None;
        }
    }

    /// Recorre una letra sin sincronizar; la vista limita el desplazamiento al dibujar.
    fn scroll_lyrics(&mut self, action: Action) {
        if let Action::Navigate(navigation) = action {
            let scroll = &mut self.state.lyrics.scroll;
            *scroll = navigation.apply(*scroll, PAGE_LINES);
        }
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

/// La línea elegida tras `navigation`: parte de la elegida o, si se seguía la canción,
/// de la que suena. Antes de la primera línea, cualquier tecla salvo `Last` elige la
/// primera. Nunca pasa de la última.
fn move_cursor(
    cursor: Option<usize>,
    current: Option<usize>,
    navigation: Navigation,
    line_count: usize,
) -> usize {
    let last = line_count.saturating_sub(1);
    match cursor.or(current) {
        Some(start) => navigation.apply(start, PAGE_LINES).min(last),
        None if navigation == Navigation::Last => last,
        None => 0,
    }
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
    fn the_cursor_starts_at_the_current_line_and_stays_inside_the_lyrics() {
        let next = Navigation::Next;
        assert_eq!(move_cursor(None, Some(4), next, 30), 5);
        assert_eq!(move_cursor(None, None, next, 30), 0);
        assert_eq!(move_cursor(None, None, Navigation::Last, 30), 29);
        assert_eq!(move_cursor(Some(10), Some(4), Navigation::Previous, 30), 9);
        assert_eq!(move_cursor(Some(29), None, next, 30), 29);
        assert_eq!(move_cursor(Some(25), None, Navigation::PageDown, 30), 29);
        assert_eq!(move_cursor(Some(12), None, Navigation::First, 30), 0);
        assert_eq!(move_cursor(None, Some(3), Navigation::Last, 30), 29);
    }

    #[test]
    fn the_current_line_is_visible_in_the_view_or_the_panel() {
        assert!(shows_current_line(ViewKind::Lyrics, false));
        assert!(!shows_current_line(ViewKind::Library, false));
        assert!(shows_current_line(ViewKind::Library, true));
    }
}
