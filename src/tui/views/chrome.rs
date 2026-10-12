//! Marco de la pantalla: bloques con título, reglas horizontales que tocan los bordes
//! y divisores verticales que se unen solos con las reglas que cruzan.

use std::ops::Range;
use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Block;

use super::format::saturating_u16;
use super::layout::{ALBUM_COLUMN_MIN_WIDTH, LayoutMode};
use super::theme;
use crate::i18n::{Catalog, Message, Phrase};
use crate::tui::action::{Action, key_label};
use crate::tui::appearance::ProgressBars;
use crate::tui::glyphs::Glyphs;

/// Lo que todas las vistas necesitan saber para dibujarse.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub glyphs: &'a Glyphs,
    pub texts: &'a dyn Catalog,
    pub mode: LayoutMode,
    /// Ancho total de la terminal.
    pub width: u16,
    pub now: Instant,
    pub progress_bars: ProgressBars,
    /// Línea actual de la letra en el panel "ahora suena".
    pub lyrics_line: bool,
}

impl Context<'_> {
    pub fn is_compact(&self) -> bool {
        self.mode == LayoutMode::Compact
    }

    /// Las tablas de temas pierden la columna Álbum en terminales angostas.
    pub fn shows_album(&self) -> bool {
        self.width >= ALBUM_COLUMN_MIN_WIDTH
    }

    pub fn phrase(&self, phrase: Phrase) -> &'static str {
        self.texts.phrase(phrase)
    }

    pub fn message(&self, message: Message<'_>) -> String {
        self.texts.message(&message)
    }

    /// La tecla principal de una acción, para sugerirla en pantalla.
    pub fn key(&self, action: Action) -> String {
        key_label(action, self.glyphs, self.texts).unwrap_or_default()
    }
}

/// Título de bloque `─ texto `, pegado al borde como en `┌─ mpctui ──`.
pub fn block_title<'a>(text: Line<'a>, glyphs: &Glyphs) -> Line<'a> {
    let mut spans = vec![
        Span::styled(glyphs.border.horizontal_top, theme::BORDER),
        Span::raw(" "),
    ];
    spans.extend(text.spans);
    spans.push(Span::raw(" "));
    Line::from(spans)
}

/// Título al final de un borde, ` texto ─`, como en `── ● localhost:6600 ─┐`.
pub fn end_title<'a>(text: Line<'a>, glyphs: &Glyphs) -> Line<'a> {
    let mut spans = vec![Span::raw(" ")];
    spans.extend(text.spans);
    spans.push(Span::raw(" "));
    spans.push(Span::styled(glyphs.border.horizontal_top, theme::BORDER));
    Line::from(spans)
}

/// Bloque con bordes del juego de glifos y título a la izquierda.
pub fn titled_block<'a>(title: Line<'a>, glyphs: &Glyphs) -> Block<'a> {
    Block::bordered()
        .border_set(glyphs.border)
        .border_style(theme::BORDER)
        .title(block_title(title, glyphs))
}

/// Regla horizontal en la fila `row` (un área interior de una sola fila) que se une a
/// los bordes izquierdo y derecho: `├───┤`. El título, si hay, va después de `├─`.
pub fn rule(buffer: &mut Buffer, row: Rect, glyphs: &Glyphs, title: Option<Line<'_>>) {
    let junction = glyphs.junction;
    let horizontal = glyphs.border.horizontal_top;
    for x in row.x..row.right() {
        set_symbol(buffer, x, row.y, horizontal);
    }
    if let Some(left) = row.x.checked_sub(1) {
        set_symbol(buffer, left, row.y, junction.tee_right);
    }
    set_symbol(buffer, row.right(), row.y, junction.tee_left);
    if let Some(title) = title {
        let title = block_title(title, glyphs);
        buffer.set_line(row.x, row.y, &title, row.width);
    }
}

/// Agrega a una regla ya dibujada un título pegado a su extremo derecho (`── texto ─┤`).
pub fn rule_end_title(buffer: &mut Buffer, row: Rect, glyphs: &Glyphs, title: Line<'_>) {
    let title = end_title(title, glyphs);
    let width = saturating_u16(title.width()).min(row.width);
    buffer.set_line(row.right() - width, row.y, &title, width);
}

/// Línea vertical entre columnas en `rows`. Donde cruza una regla pone `┼`; abajo se
/// une a la regla que cierra la vista (`┴`) y, si `join_top`, arriba también (`┬`).
pub fn divider(buffer: &mut Buffer, x: u16, rows: Range<u16>, glyphs: &Glyphs, join_top: bool) {
    let horizontal = glyphs.border.horizontal_top;
    let junction = glyphs.junction;
    for y in rows.clone() {
        let symbol = if symbol_at(buffer, x, y) == Some(horizontal) {
            junction.cross
        } else {
            glyphs.border.vertical_left
        };
        set_symbol(buffer, x, y, symbol);
    }
    if join_top
        && let Some(above) = rows.start.checked_sub(1)
        && symbol_at(buffer, x, above) == Some(horizontal)
    {
        set_symbol(buffer, x, above, junction.tee_down);
    }
    if symbol_at(buffer, x, rows.end) == Some(horizontal) {
        set_symbol(buffer, x, rows.end, junction.tee_up);
    }
}

fn symbol_at(buffer: &Buffer, x: u16, y: u16) -> Option<&str> {
    buffer.cell((x, y)).map(|cell| cell.symbol())
}

fn set_symbol(buffer: &mut Buffer, x: u16, y: u16, symbol: &str) {
    if let Some(cell) = buffer.cell_mut((x, y)) {
        cell.set_symbol(symbol).set_style(theme::BORDER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::ASCII;

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    #[test]
    fn rules_and_dividers_join() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 9, 5));
        rule(&mut buffer, Rect::new(1, 0, 7, 1), &ASCII, None);
        rule(&mut buffer, Rect::new(1, 2, 7, 1), &ASCII, None);
        rule(
            &mut buffer,
            Rect::new(1, 4, 7, 1),
            &ASCII,
            Some(Line::from("t")),
        );
        divider(&mut buffer, 4, 1..4, &ASCII, true);
        assert_eq!(row_text(&buffer, 0), "+---+---+");
        assert_eq!(row_text(&buffer, 1), "    |    ");
        assert_eq!(row_text(&buffer, 2), "+---+---+");
        assert_eq!(row_text(&buffer, 4), "+- t ---+");
        rule_end_title(&mut buffer, Rect::new(1, 4, 7, 1), &ASCII, Line::from("x"));
        assert_eq!(row_text(&buffer, 4), "+- t x -+");
    }
}
