//! Vista Visual: el visualizador corre solo mientras la vista está activa, con la
//! forma y la resolución que pidió el último dibujado. `6` entra o cambia de forma.

use super::{App, Backend};
use crate::i18n::Message;
use crate::service::{RunningVisualizer, VisualState};
use crate::tui::action::{Action, key_label};
use crate::tui::state::ViewKind;
use crate::tui::views::visualizer_style_name;

impl<B: Backend> App<B> {
    /// Desde otra vista entra con la última forma usada; en el visualizador pasa a la
    /// siguiente y lo avisa.
    pub(super) async fn show_visualizer(&mut self) {
        if self.state.view != ViewKind::Visualizer {
            return self.show_view(ViewKind::Visualizer).await;
        }
        let visual = &mut self.state.visual;
        visual.style = visual.style.next(visual.channels);
        let texts = self.appearance.texts;
        let key =
            key_label(Action::ShowVisualizer, self.appearance.glyphs, texts).unwrap_or_default();
        let style = texts.phrase(visualizer_style_name(visual.style));
        self.notify_info(self.message(Message::VisualizerChanged { style, key: &key }));
    }

    /// Se llama después de cada dibujado: lanza o cancela el visualizador según la
    /// vista activa y le pasa lo que pidió la vista (forma y tamaño).
    pub(super) fn sync_visualizer(&mut self) {
        if self.state.view != ViewKind::Visualizer {
            if self.visualizer.take().is_some() {
                self.state.visual.state = VisualState::default();
            }
            return;
        }
        let Some(request) = self.state.visual.request else {
            return;
        };
        match &self.visualizer {
            Some(running) if running.request() != request => running.set_request(request),
            Some(_) => {}
            None => self.visualizer = Some(self.services.visualizer.launch(request)),
        }
    }
}

/// El próximo estado del visualizador; sin visualizador, espera para siempre.
pub(super) async fn next_state(visualizer: &mut Option<RunningVisualizer>) -> Option<VisualState> {
    match visualizer {
        Some(running) => running.next_state().await,
        None => std::future::pending().await,
    }
}
