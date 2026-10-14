//! Búsqueda (`4`): texto y campo, resumen, y resultados agrupados con las coincidencias
//! resaltadas.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use super::{GroupedLook, draw_grouped, draw_line, text_row};
use crate::domain::Tag;
use crate::i18n::{Message, Phrase};
use crate::tui::action::Action;
use crate::tui::prompt::PromptKind;
use crate::tui::state::{AppState, SEARCH_FIELDS, SearchView};
use crate::tui::theme::Role;
use crate::tui::views::chrome::{self, Context};
use crate::tui::views::format::tag_label;
use crate::tui::views::widgets::{SongColumns, spread};

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let [query_area, summary_area, rule_area, list_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let editing = state
        .prompt
        .as_ref()
        .filter(|prompt| prompt.kind == PromptKind::Search);
    let search = &mut state.search;
    let last_query = search.text.as_ref().map(|text| text.as_str());
    let shown = editing.map_or(last_query.unwrap_or_default(), |prompt| {
        prompt.input.as_str()
    });
    let accent = context.style(Role::Accent);
    let mut query = vec![
        Span::styled(format!("{}: ", context.phrase(Phrase::Search)), accent),
        Span::styled(shown.to_owned(), accent),
    ];
    if editing.is_some() {
        query.push(Span::styled(context.glyphs.input_cursor, accent));
    }
    let query_area = text_row(query_area);
    let fields = fields_line(search.field, context);
    let line = spread(Line::from(query), fields, query_area.width, context.glyphs);
    draw_line(frame, query_area, &line);
    let summary = summary_line(search, last_query, context);
    draw_line(frame, text_row(summary_area), &summary);
    chrome::rule(frame.buffer_mut(), rule_area, context, None);
    let look = GroupedLook {
        columns: SongColumns {
            position: false,
            album: context.shows_album(),
        },
        now_playing: None,
        highlight: last_query,
        selectable: true,
        header: None,
    };
    draw_grouped(frame, list_area, context, &mut search.results, look);
}

/// `campo: (todo)  artista   álbum   título  Tab cambia`.
fn fields_line(active: Tag, context: &Context<'_>) -> Line<'static> {
    let field_label = context.phrase(Phrase::FieldLabel);
    let muted = context.style(Role::Muted);
    let mut spans = vec![Span::styled(format!("{field_label}:"), muted)];
    for field in SEARCH_FIELDS {
        let label = context.phrase(tag_label(field)).to_lowercase();
        spans.push(if field == active {
            Span::styled(format!(" ({label})"), context.strong(Role::Accent))
        } else {
            Span::styled(format!("  {label} "), muted)
        });
    }
    let hint = context.phrase(Phrase::FieldHint);
    spans.push(Span::styled(format!("  {hint}"), muted));
    Line::from(spans)
}

fn summary_line(search: &SearchView, query: Option<&str>, context: &Context<'_>) -> Line<'static> {
    let results = search.results.items().len();
    let text = match query {
        None => {
            let key = context.key(Action::StartSearch);
            context.message(Message::SearchHint { key: &key })
        }
        Some(query) if results == 0 => context.message(Message::NoResults { query }),
        Some(_) => context.message(Message::ResultsInAlbums {
            results,
            albums: search.results.timeline().runs.len(),
        }),
    };
    Line::styled(text, context.style(Role::Muted))
}
