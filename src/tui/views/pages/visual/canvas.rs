//! Lienzo de puntos de la Onda y el Vectorscopio. En Unicode cada celda es braille de
//! 2 × 4 puntos; en ASCII, una columna con sub-niveles (`'`, `-`, `.`) y `|` cuando
//! una celda tiene puntos en más de un sub-nivel.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::tui::glyphs::{BRAILLE_BASE, CellPlot, Plot};

/// Puntos braille por celda.
const BRAILLE_COLUMNS: usize = 2;
const BRAILLE_ROWS: usize = 4;
/// Bit de cada punto braille, por columna y fila dentro de la celda.
const BRAILLE_BITS: [[u32; BRAILLE_ROWS]; BRAILLE_COLUMNS] =
    [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// Puntos por celda, a lo ancho y a lo alto.
pub fn dots_per_cell(plot: Plot) -> (usize, usize) {
    match plot {
        Plot::Braille => (BRAILLE_COLUMNS, BRAILLE_ROWS),
        Plot::Cells(cells) => (1, cells.levels.len()),
    }
}

/// Lo que se sabe de una celda al pintarla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasCell {
    /// Columna de la celda, desde la izquierda del lienzo.
    pub column: usize,
    /// Puntos encendidos y puntos que entran en la celda.
    pub dots: usize,
    pub capacity: usize,
}

impl CanvasCell {
    /// Qué tan llena está, entre 0 y 1.
    pub fn density(self) -> f32 {
        self.dots as f32 / self.capacity.max(1) as f32
    }
}

pub struct DotCanvas {
    area: Rect,
    plot: Plot,
    columns: usize,
    rows: usize,
    dots: Vec<bool>,
}

impl DotCanvas {
    pub fn new(area: Rect, plot: Plot) -> Self {
        let (cell_columns, cell_rows) = dots_per_cell(plot);
        let columns = usize::from(area.width) * cell_columns;
        let rows = usize::from(area.height) * cell_rows;
        Self {
            area,
            plot,
            columns,
            rows,
            dots: vec![false; columns * rows],
        }
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Enciende un punto; fuera del lienzo no hace nada.
    pub fn set(&mut self, column: usize, row: usize) {
        if column < self.columns && row < self.rows {
            self.dots[row * self.columns + column] = true;
        }
    }

    fn is_set(&self, column: usize, row: usize) -> bool {
        self.dots[row * self.columns + column]
    }

    /// Dibuja solo las celdas con algún punto: lo de abajo queda a la vista. El color
    /// de cada celda lo decide `style_of` (por columna o por cantidad de puntos).
    pub fn draw(&self, buffer: &mut Buffer, style_of: impl Fn(CanvasCell) -> Style) {
        let (cell_columns, cell_rows) = dots_per_cell(self.plot);
        for cell_y in 0..self.area.height {
            for cell_x in 0..self.area.width {
                let first_column = usize::from(cell_x) * cell_columns;
                let first_row = usize::from(cell_y) * cell_rows;
                let drawn = match self.plot {
                    Plot::Braille => self.braille(first_column, first_row),
                    Plot::Cells(cells) => self.cell_text(cells, first_column, first_row),
                };
                if let Some((symbol, dots)) = drawn {
                    let cell = CanvasCell {
                        column: usize::from(cell_x),
                        dots,
                        capacity: cell_columns * cell_rows,
                    };
                    let x = self.area.x + cell_x;
                    buffer.set_string(x, self.area.y + cell_y, symbol, style_of(cell));
                }
            }
        }
    }

    /// El carácter braille de la celda y cuántos puntos tiene.
    fn braille(&self, first_column: usize, first_row: usize) -> Option<(String, usize)> {
        let mut bits = 0;
        let mut dots = 0;
        for (column, column_bits) in BRAILLE_BITS.iter().enumerate() {
            for (row, bit) in column_bits.iter().enumerate() {
                if self.is_set(first_column + column, first_row + row) {
                    bits |= bit;
                    dots += 1;
                }
            }
        }
        (bits > 0)
            .then(|| char::from_u32(BRAILLE_BASE + bits))
            .flatten()
            .map(|character| (String::from(character), dots))
    }

    fn cell_text(
        &self,
        cells: CellPlot,
        column: usize,
        first_row: usize,
    ) -> Option<(String, usize)> {
        let set: Vec<usize> = (0..cells.levels.len())
            .filter(|level| self.is_set(column, first_row + level))
            .collect();
        let symbol = match set.as_slice() {
            [] => return None,
            [level] => cells.levels[*level],
            _ => cells.join,
        };
        Some((symbol.to_owned(), set.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::{ASCII, UNICODE};

    fn drawn(canvas: &DotCanvas, area: Rect) -> String {
        let mut buffer = Buffer::empty(area);
        canvas.draw(&mut buffer, |_| Style::new());
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    #[test]
    fn braille_dots_become_their_bits() {
        let area = Rect::new(0, 0, 2, 1);
        let mut canvas = DotCanvas::new(area, UNICODE.plot);
        assert_eq!((canvas.columns(), canvas.rows()), (4, 4));
        canvas.set(0, 0);
        canvas.set(1, 3);
        canvas.set(9, 9);
        assert_eq!(drawn(&canvas, area), "⢁ ");
    }

    #[test]
    fn each_cell_knows_its_column_and_dots() {
        let area = Rect::new(0, 0, 2, 1);
        let mut canvas = DotCanvas::new(area, UNICODE.plot);
        canvas.set(2, 0);
        canvas.set(3, 1);
        let cells = std::cell::RefCell::new(Vec::new());
        canvas.draw(&mut Buffer::empty(area), |cell| {
            cells.borrow_mut().push(cell);
            Style::new()
        });
        let cell = CanvasCell {
            column: 1,
            dots: 2,
            capacity: 8,
        };
        assert_eq!(cells.into_inner(), [cell]);
        assert_eq!(cell.density(), 0.25);
    }

    #[test]
    fn ascii_cells_use_sub_levels_and_join_jumps() {
        let area = Rect::new(0, 0, 3, 1);
        let mut canvas = DotCanvas::new(area, ASCII.plot);
        assert_eq!((canvas.columns(), canvas.rows()), (3, 3));
        canvas.set(0, 0);
        canvas.set(1, 2);
        canvas.set(2, 0);
        canvas.set(2, 1);
        assert_eq!(drawn(&canvas, area), "'.|");
    }
}
