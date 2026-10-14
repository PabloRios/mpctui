//! Tema de colores en vivo: `t` pasa al siguiente sin reiniciar. No se guarda al
//! salir; el de arranque se elige con `theme`.

use super::{App, Backend};
use crate::config::ThemeId;
use crate::i18n::{Message, Phrase};
use crate::tui::action::Action;
use crate::tui::theme::Theme;

impl<B: Backend> App<B> {
    /// Con 16 colores o sin color los temas no se ven: no cambia nada y lo avisa.
    pub(super) fn next_theme(&mut self) {
        let current = &self.appearance.theme;
        if !current.depth().shows_themes() {
            return self.notify_info(self.phrase(Phrase::ThemesNeedColors));
        }
        let next = Theme::resolve(current.id().next(), current.depth());
        let theme = next.id().name();
        let key = self.key(Action::NextTheme);
        let text = self.message(Message::ThemeChanged { theme, key: &key });
        self.appearance.theme = next;
        self.notify_info(text);
    }

    /// Al arrancar con un tema que la terminal no puede mostrar, avisa que se usa
    /// `terminal`.
    pub(super) fn explain_unavailable_theme(&mut self) {
        let theme = &self.appearance.theme;
        if theme.depth().shows_themes() || theme.id() == ThemeId::Terminal {
            return;
        }
        let name = theme.id().name();
        self.notify_info(self.message(Message::ThemeNeedsColors { theme: name }));
    }
}
