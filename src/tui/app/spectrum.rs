//! Vista Espectro: el visualizador corre solo mientras la vista está activa, con la
//! cantidad de bandas que entra en la terminal.

use ratatui::layout::Rect;

use super::{App, Backend};
use crate::service::{RunningVisualizer, SpectrumState};
use crate::tui::state::ViewKind;
use crate::tui::views;

impl<B: Backend> App<B> {
    /// Se llama después de cada dibujado: lanza o cancela el visualizador según la
    /// vista activa y le avisa si cambió el ancho.
    pub(super) fn sync_visualizer(&mut self, terminal: Rect) {
        if self.state.view != ViewKind::Spectrum {
            if self.visualizer.take().is_some() {
                self.state.spectrum.state = SpectrumState::default();
            }
            return;
        }
        let bands = views::spectrum_bands(terminal);
        match &self.visualizer {
            Some(running) if running.bands() != bands => running.set_bands(bands),
            Some(_) => {}
            None => self.visualizer = Some(self.services.visualizer.launch(bands)),
        }
    }
}

/// El próximo estado del visualizador; sin visualizador, espera para siempre.
pub(super) async fn next_state(
    visualizer: &mut Option<RunningVisualizer>,
) -> Option<SpectrumState> {
    match visualizer {
        Some(running) => running.next_state().await,
        None => std::future::pending().await,
    }
}
