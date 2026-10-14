//! Directorios (`3`): migas de pan desde la raíz y entradas con insignia.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use super::{breadcrumbs, draw_line, draw_list, text_row};
use crate::domain::{Entry, PATH_SEPARATOR};
use crate::i18n::Message;
use crate::tui::state::AppState;
use crate::tui::theme::Role;
use crate::tui::views::chrome::{self, Context};
use crate::tui::views::format::format_duration;
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
    let count = Line::styled(
        context.message(Message::EntryCount(browse.entries.items().len())),
        context.style(Role::Muted),
    );
    let crumbs_area = text_row(crumbs_area);
    let available = usize::from(crumbs_area.width).saturating_sub(count.width() + 1);
    let root = std::iter::once(ROOT_LABEL);
    let crumbs = breadcrumbs(root.chain(segments), available, context);
    let line = spread(crumbs, count, crumbs_area.width, context.glyphs);
    draw_line(frame, crumbs_area, &line);
    chrome::rule(frame.buffer_mut(), rule_area, context, None);
    let rows_context = *context;
    draw_list(
        frame,
        list_area,
        context,
        &mut browse.entries,
        true,
        |entries| EntryRows {
            entries,
            context: rows_context,
        },
    );
}

struct EntryRows<'a> {
    entries: &'a [Entry],
    context: Context<'a>,
}

impl RowSource for EntryRows<'_> {
    fn row_count(&self) -> usize {
        self.entries.len()
    }

    /// Directorios con `/` al final; temas con su duración a la derecha.
    fn row(&self, index: usize, width: u16) -> ListRow<'_> {
        let entry = &self.entries[index];
        let context = &self.context;
        let glyphs = context.glyphs;
        // Insignia y nombre: los directorios como en `ls`; en los temas, la insignia en
        // `accent` y el nombre como texto común.
        let (badge, badge_role, name_role, suffix, duration) = match entry {
            Entry::Directory { .. } => (
                glyphs.directory,
                Role::Directory,
                Role::Directory,
                DIRECTORY_SUFFIX,
                None,
            ),
            Entry::File(song) => (
                glyphs.now_playing,
                Role::Accent,
                Role::Text,
                "",
                song.duration,
            ),
            Entry::Playlist { .. } => (glyphs.playlist, Role::Playlist, Role::Playlist, "", None),
        };
        let name_style = context.style(name_role);
        let name = Line::from(vec![
            Span::styled(badge, context.style(badge_role)),
            Span::raw(BADGE_GAP),
            Span::styled(entry.name(), name_style),
            Span::styled(suffix, name_style),
        ]);
        let duration = Line::styled(
            duration.map(format_duration).unwrap_or_default(),
            context.style(Role::Duration),
        );
        ListRow::Item(spread(name, duration, width, glyphs))
    }
}
