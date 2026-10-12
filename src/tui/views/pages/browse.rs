//! Directorios (`3`): migas de pan desde la raíz y entradas con insignia.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use super::{breadcrumbs, draw_line, draw_list, text_row};
use crate::domain::{Entry, PATH_SEPARATOR};
use crate::tui::glyphs::Glyphs;
use crate::tui::state::AppState;
use crate::tui::views::chrome::{self, Context};
use crate::tui::views::format::{fit_start, format_duration, plural};
use crate::tui::views::theme;
use crate::tui::views::widgets::{ListRow, RowSource, spread};

/// Raíz de la biblioteca en las migas de pan.
const ROOT_LABEL: &str = "/";
/// Separación entre una insignia y el nombre.
const BADGE_GAP: &str = " ";
/// Marca de directorio después del nombre.
const DIRECTORY_SUFFIX: &str = "/";

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let [crumbs_area, rule_area, list_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let browse = &mut state.browse;
    let path = browse.path.clone();
    let segments = path
        .split(PATH_SEPARATOR)
        .filter(|segment| !segment.is_empty());
    let crumbs = breadcrumbs(std::iter::once(ROOT_LABEL).chain(segments), context);
    let count = Line::styled(
        plural(browse.entries.items().len(), "entrada", "entradas"),
        theme::MUTED,
    );
    let crumbs_area = text_row(crumbs_area);
    let available = usize::from(crumbs_area.width).saturating_sub(count.width() + 1);
    let crumbs = Line::from(fit_start(&crumbs, available, context.glyphs.ellipsis));
    let line = spread(crumbs, count, crumbs_area.width, context.glyphs);
    draw_line(frame, crumbs_area, &line);
    chrome::rule(frame.buffer_mut(), rule_area, context.glyphs, None);
    let glyphs = context.glyphs;
    draw_list(
        frame,
        list_area,
        context,
        &mut browse.entries,
        true,
        |entries| EntryRows { entries, glyphs },
    );
}

struct EntryRows<'a> {
    entries: &'a [Entry],
    glyphs: &'a Glyphs,
}

impl RowSource for EntryRows<'_> {
    fn row_count(&self) -> usize {
        self.entries.len()
    }

    /// Directorios con `/` al final; temas con su duración a la derecha.
    fn row(&self, index: usize, width: u16) -> ListRow<'_> {
        let entry = &self.entries[index];
        let (badge, style, suffix, duration) = match entry {
            Entry::Directory { .. } => {
                (self.glyphs.directory, theme::ACCENT, DIRECTORY_SUFFIX, None)
            }
            Entry::File(song) => (self.glyphs.now_playing, theme::PLAIN, "", song.duration),
            Entry::Playlist { .. } => (self.glyphs.playlist, theme::MUTED, "", None),
        };
        let name = Line::from(vec![
            Span::styled(badge, style),
            Span::raw(BADGE_GAP),
            Span::styled(entry.name(), style),
            Span::styled(suffix, style),
        ]);
        let duration = Line::styled(
            duration.map(format_duration).unwrap_or_default(),
            theme::MUTED,
        );
        ListRow::Item(spread(name, duration, width, self.glyphs))
    }
}
