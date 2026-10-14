//! Ayuda superpuesta en dos columnas, agrupada por sección y generada desde la tabla
//! de atajos. Debe entrar en 30 filas.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::chrome::{Context, titled_block};
use super::format::{fit_line, saturating_u16};
use crate::i18n::Phrase;
use crate::tui::action::{HelpSection, help_sections};
use crate::tui::theme::Role;
/// Sangría de las entradas bajo el título de su sección.
const ENTRY_INDENT: &str = "  ";
const KEYS_GAP: usize = 2;
const COLUMN_GAP: usize = 3;
/// Bordes, fila en blanco arriba, fila en blanco y pista de cierre abajo.
const FIXED_ROWS: usize = 5;
/// Columnas de los bordes y el margen interior.
const FIXED_COLUMNS: usize = 4;

pub fn render(frame: &mut Frame, area: Rect, context: &Context<'_>) {
    let glyphs = context.glyphs;
    let sections = help_sections(glyphs, context.texts);
    let keys_width = sections
        .iter()
        .flat_map(|section| &section.entries)
        .map(|entry| Span::raw(entry.keys.as_str()).width())
        .max()
        .unwrap_or_default();
    let needed = sections
        .iter()
        .flat_map(|section| &section.entries)
        .map(|entry| {
            Span::raw(ENTRY_INDENT).width()
                + keys_width
                + KEYS_GAP
                + Span::raw(context.phrase(entry.description)).width()
        })
        .max()
        .unwrap_or_default();
    let available = usize::from(area.width).saturating_sub(FIXED_COLUMNS + COLUMN_GAP) / 2;
    let column_width = needed.min(available);
    let (left, right) = split_columns(&sections);
    let left = column_lines(left, keys_width, column_width, context);
    let right = column_lines(right, keys_width, column_width, context);
    let body_rows = left.len().max(right.len());
    let mut lines = vec![Line::default()];
    for row in 0..body_rows {
        let mut spans = vec![Span::raw(" ")];
        let left_line = left.get(row).cloned().unwrap_or_default();
        let padding = column_width.saturating_sub(left_line.width()) + COLUMN_GAP;
        spans.extend(left_line.spans);
        spans.push(Span::raw(" ".repeat(padding)));
        spans.extend(right.get(row).cloned().unwrap_or_default().spans);
        lines.push(Line::from(spans));
    }
    lines.push(Line::default());
    let close_hint = context.phrase(Phrase::HelpClose);
    lines.push(Line::styled(format!("{close_hint} "), context.style(Role::Muted)).right_aligned());
    let width = (column_width * 2 + COLUMN_GAP + FIXED_COLUMNS).min(usize::from(area.width));
    let height = (body_rows + FIXED_ROWS).min(usize::from(area.height));
    let popup = area.centered(
        Constraint::Length(saturating_u16(width)),
        Constraint::Length(saturating_u16(height)),
    );
    frame.render_widget(Clear, popup);
    let title = context.phrase(Phrase::HelpTitle);
    // Es la ventana con foco: el borde va en `accent`.
    let title = Line::styled(title, context.style(Role::TextStrong));
    let block = titled_block(title, Role::Accent, context);
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

/// Corta las secciones (en orden) donde la columna más alta queda lo más baja posible.
fn split_columns(sections: &[HelpSection]) -> (&[HelpSection], &[HelpSection]) {
    let rows = |section: &HelpSection| section.entries.len() + 2;
    let total: usize = sections.iter().map(rows).sum();
    let split = (0..=sections.len())
        .min_by_key(|split| {
            let left: usize = sections[..*split].iter().map(rows).sum();
            left.max(total - left)
        })
        .unwrap_or_default();
    sections.split_at(split)
}

/// Título de cada sección, sus entradas y una fila en blanco entre secciones.
fn column_lines(
    sections: &[HelpSection],
    keys_width: usize,
    column_width: usize,
    context: &Context<'_>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        if index > 0 {
            lines.push(Line::default());
        }
        lines.push(Line::styled(
            context.phrase(section.group.title()),
            context.style(Role::AccentAlt),
        ));
        for entry in &section.entries {
            let line = Line::from(vec![
                Span::raw(ENTRY_INDENT),
                Span::styled(
                    format!("{:<keys_width$}", entry.keys),
                    context.strong(Role::Accent),
                ),
                Span::raw(" ".repeat(KEYS_GAP)),
                Span::raw(context.phrase(entry.description)),
            ]);
            lines.push(fit_line(line, column_width, context.glyphs.ellipsis));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Spanish;
    use crate::tui::glyphs::UNICODE;

    /// Filas de una columna: título y entradas de cada sección y una en blanco entre
    /// secciones.
    fn column_rows(sections: &[HelpSection]) -> usize {
        let entries: usize = sections
            .iter()
            .map(|section| section.entries.len() + 1)
            .sum();
        entries + sections.len().saturating_sub(1)
    }

    #[test]
    fn the_help_fits_in_thirty_rows() {
        let sections = help_sections(&UNICODE, &Spanish);
        let (left, right) = split_columns(&sections);
        assert!(!left.is_empty() && !right.is_empty());
        let body = column_rows(left).max(column_rows(right));
        assert!(body + FIXED_ROWS <= 30, "{body}");
    }
}
