//! Barras segmentadas y medidores. El algoritmo devuelve celdas abstractas; un segundo
//! paso las traduce a glifos y estilos. Así se testea sin ratatui ni glifos.

use std::ops::Range;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::theme;
use crate::tui::glyphs::Glyphs;

/// Mínimo de celdas por tema en la barra del álbum.
pub const ALBUM_BAR_MIN_CELLS: usize = 1;
/// Mínimo de celdas por álbum en la barra de la cola.
pub const QUEUE_BAR_MIN_CELLS: usize = 3;
/// Álbumes visibles a cada lado del actual en la barra de la cola.
pub const QUEUE_BAR_WINDOW_RADIUS: usize = 3;
/// Por debajo de este ancho la barra no se dibuja.
pub const BAR_MIN_WIDTH: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarCell {
    Filled,
    Head,
    Empty,
    Separator,
    /// Hay segmentos fuera de la ventana visible.
    More,
    /// Duración desconocida.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterCell {
    Filled,
    Empty,
}

/// Cómo se reparte una barra entre sus segmentos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarPolicy {
    pub min_cells: usize,
    /// Segmentos a cada lado del actual; `None` = todos los que entren.
    pub window_radius: Option<usize>,
}

pub const SONG_BAR: BarPolicy = BarPolicy {
    min_cells: 1,
    window_radius: None,
};
pub const ALBUM_BAR: BarPolicy = BarPolicy {
    min_cells: ALBUM_BAR_MIN_CELLS,
    window_radius: None,
};
pub const QUEUE_BAR: BarPolicy = BarPolicy {
    min_cells: QUEUE_BAR_MIN_CELLS,
    window_radius: Some(QUEUE_BAR_WINDOW_RADIUS),
};

/// Contenido de una barra: peso de cada segmento y posición, en la misma unidad
/// (milisegundos, o temas en el modo por cantidad).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarInput {
    pub weights: Vec<u64>,
    /// `None` = detenido: sin cabeza.
    pub position: Option<u64>,
}

/// Barra completa en `width` columnas. `more_width` es el ancho del glifo `bar_more`.
pub fn segmented_bar(
    input: &BarInput,
    width: usize,
    policy: BarPolicy,
    more_width: usize,
) -> Vec<BarCell> {
    let weights = &input.weights;
    if weights.is_empty() {
        return vec![BarCell::Empty; width];
    }
    let current = input
        .position
        .map_or(0, |position| current_segment(weights, position));
    let window = choose_window(weights.len(), current, width, policy, more_width);
    let fixed = fixed_columns(&window, weights.len(), more_width);
    let sizes = allocate_cells(
        &weights[window.clone()],
        width.saturating_sub(fixed),
        policy.min_cells,
    );
    let mut cells = Vec::with_capacity(width);
    if window.start > 0 {
        cells.extend([BarCell::More, BarCell::Separator]);
    }
    let mut segment_start: u64 = weights[..window.start].iter().sum();
    for (offset, size) in sizes.into_iter().enumerate() {
        let index = window.start + offset;
        if offset > 0 {
            cells.push(BarCell::Separator);
        }
        let segment = Segment {
            start: segment_start,
            weight: weights[index],
            cells: size,
        };
        paint_segment(&mut cells, segment, index.cmp(&current), input.position);
        segment_start += weights[index];
    }
    if window.end < weights.len() {
        cells.extend([BarCell::Separator, BarCell::More]);
    }
    fit_columns(cells, width, more_width)
}

/// Índice del segmento que contiene la posición (el último si ya pasó el final).
fn current_segment(weights: &[u64], position: u64) -> usize {
    let mut end = 0;
    for (index, weight) in weights.iter().enumerate() {
        end += weight;
        if position < end {
            return index;
        }
    }
    weights.len() - 1
}

/// Achica el radio hasta que todos los segmentos de la ventana tengan su mínimo.
fn choose_window(
    count: usize,
    current: usize,
    width: usize,
    policy: BarPolicy,
    more_width: usize,
) -> Range<usize> {
    let mut radius = policy.window_radius.unwrap_or(count);
    loop {
        let window = current.saturating_sub(radius)..(current + radius + 1).min(count);
        let required = window.len() * policy.min_cells + fixed_columns(&window, count, more_width);
        if required <= width || radius == 0 {
            return window;
        }
        radius -= 1;
    }
}

/// Separadores entre segmentos y, por cada extremo recortado, `bar_more` + separador.
fn fixed_columns(window: &Range<usize>, count: usize, more_width: usize) -> usize {
    let separators = window.len().saturating_sub(1);
    let cut_ends = usize::from(window.start > 0) + usize::from(window.end < count);
    separators + cut_ends * (more_width + 1)
}

/// Reparto proporcional por mayor resto, respetando el mínimo por segmento.
pub fn allocate_cells(weights: &[u64], cells: usize, min_cells: usize) -> Vec<usize> {
    let total: u128 = weights.iter().map(|weight| u128::from(*weight)).sum();
    // Sin pesos, todos valen lo mismo.
    let weights: Vec<u128> = if total == 0 {
        vec![1; weights.len()]
    } else {
        weights.iter().map(|weight| u128::from(*weight)).collect()
    };
    let total: u128 = weights.iter().sum();
    let cells_wide = cells as u128;
    let numerators: Vec<u128> = weights.iter().map(|weight| weight * cells_wide).collect();
    let mut sizes: Vec<usize> = numerators
        .iter()
        .map(|numerator| {
            usize::try_from(numerator / total)
                .unwrap_or(cells)
                .max(min_cells)
        })
        .collect();
    let mut assigned: usize = sizes.iter().sum();
    let mut by_remainder: Vec<usize> = (0..weights.len()).collect();
    by_remainder.sort_by_key(|index| std::cmp::Reverse(numerators[*index] % total));
    for index in by_remainder
        .iter()
        .cycle()
        .take(cells.saturating_sub(assigned))
    {
        sizes[*index] += 1;
    }
    assigned = assigned.max(cells);
    while assigned > cells {
        let excess =
            |index: usize| sizes[index] as i128 * total as i128 - numerators[index] as i128;
        let Some(largest) = (0..sizes.len())
            .filter(|index| sizes[*index] > min_cells)
            .max_by_key(|index| excess(*index))
        else {
            break;
        };
        sizes[largest] -= 1;
        assigned -= 1;
    }
    sizes
}

/// Un segmento ya ubicado en la barra.
#[derive(Debug, Clone, Copy)]
struct Segment {
    start: u64,
    weight: u64,
    cells: usize,
}

/// Lleno si ya terminó, vacío si no empezó; el actual lleva la cabeza.
fn paint_segment(
    cells: &mut Vec<BarCell>,
    segment: Segment,
    order: std::cmp::Ordering,
    position: Option<u64>,
) {
    let Some(position) = position else {
        return cells.extend(std::iter::repeat_n(BarCell::Empty, segment.cells));
    };
    match order {
        std::cmp::Ordering::Less => {
            cells.extend(std::iter::repeat_n(BarCell::Filled, segment.cells));
        }
        std::cmp::Ordering::Greater => {
            cells.extend(std::iter::repeat_n(BarCell::Empty, segment.cells));
        }
        std::cmp::Ordering::Equal if segment.cells > 0 => {
            let head = head_cell(segment, position);
            cells.extend(std::iter::repeat_n(BarCell::Filled, head));
            cells.push(BarCell::Head);
            cells.extend(std::iter::repeat_n(
                BarCell::Empty,
                segment.cells - head - 1,
            ));
        }
        std::cmp::Ordering::Equal => {}
    }
}

/// `min(floor(fracción × celdas), celdas − 1)`.
fn head_cell(segment: Segment, position: u64) -> usize {
    let last = segment.cells - 1;
    if segment.weight == 0 {
        return last;
    }
    let progressed = u128::from(position.saturating_sub(segment.start));
    let cell = progressed * segment.cells as u128 / u128::from(segment.weight);
    usize::try_from(cell).unwrap_or(last).min(last)
}

/// Si el mínimo por segmento no dejó entrar todo, se recorta el final.
fn fit_columns(cells: Vec<BarCell>, width: usize, more_width: usize) -> Vec<BarCell> {
    let mut used = 0;
    cells
        .into_iter()
        .take_while(|cell| {
            used += if *cell == BarCell::More {
                more_width
            } else {
                1
            };
            used <= width
        })
        .collect()
}

/// `round(valor × celdas / máximo)` celdas llenas.
pub fn meter(value: u64, maximum: u64, cells: usize) -> Vec<MeterCell> {
    let filled = if maximum == 0 {
        0
    } else {
        let doubled = u128::from(value) * cells as u128 * 2;
        let rounded = (doubled + u128::from(maximum)) / (2 * u128::from(maximum));
        usize::try_from(rounded).unwrap_or(cells).min(cells)
    };
    let mut result = vec![MeterCell::Filled; filled];
    result.resize(cells, MeterCell::Empty);
    result
}

/// Columnas que ocupa un medidor de `cells` celdas con sus extremos.
pub fn meter_width(cells: usize, glyphs: &Glyphs) -> usize {
    cells + Span::raw(glyphs.bar_open).width() + Span::raw(glyphs.bar_close).width()
}

fn bar_cell_look(cell: BarCell, glyphs: &Glyphs) -> (&'static str, Style) {
    match cell {
        BarCell::Filled => (glyphs.bar_filled, theme::BAR_FILLED),
        BarCell::Head => (glyphs.bar_head, theme::BAR_HEAD),
        BarCell::Empty => (glyphs.bar_empty, theme::BAR_EMPTY),
        BarCell::Separator => (glyphs.bar_separator, theme::BAR_SEPARATOR),
        BarCell::More => (glyphs.bar_more, theme::MUTED),
        BarCell::Unknown => (glyphs.bar_unknown, theme::MUTED),
    }
}

fn meter_cell_look(cell: MeterCell, glyphs: &Glyphs) -> (&'static str, Style) {
    match cell {
        MeterCell::Filled => (glyphs.meter_filled, theme::BAR_FILLED),
        MeterCell::Empty => (glyphs.meter_empty, theme::BAR_EMPTY),
    }
}

/// Traduce celdas a una línea con extremos, juntando las celdas iguales en un tramo.
fn cells_line<T: Copy + PartialEq>(
    cells: &[T],
    glyphs: &Glyphs,
    look: impl Fn(T, &Glyphs) -> (&'static str, Style),
) -> Line<'static> {
    let mut spans = vec![Span::styled(glyphs.bar_open, theme::MUTED)];
    for group in cells.chunk_by(|left, right| left == right) {
        let (symbol, style) = look(group[0], glyphs);
        spans.push(Span::styled(symbol.repeat(group.len()), style));
    }
    spans.push(Span::styled(glyphs.bar_close, theme::MUTED));
    Line::from(spans)
}

pub fn bar_line(cells: &[BarCell], glyphs: &Glyphs) -> Line<'static> {
    cells_line(cells, glyphs, bar_cell_look)
}

pub fn meter_line(cells: &[MeterCell], glyphs: &Glyphs) -> Line<'static> {
    cells_line(cells, glyphs, meter_cell_look)
}

/// Barra de un stream: `bar_unknown` con la etiqueta centrada si entra.
pub fn unknown_bar_line(width: usize, label: &str, glyphs: &Glyphs) -> Line<'static> {
    let label_width = Span::raw(label).width();
    let mut cells = vec![BarCell::Unknown; width];
    if label_width >= width {
        return bar_line(&cells, glyphs);
    }
    let left = (width - label_width) / 2;
    let mut line = bar_line(&cells[..left], glyphs);
    // Se quita el cierre para continuar la barra después de la etiqueta.
    line.spans.pop();
    line.spans
        .push(Span::styled(label.to_owned(), theme::MUTED));
    cells.truncate(width - left - label_width);
    let mut right = bar_line(&cells, glyphs).spans;
    right.remove(0);
    line.spans.extend(right);
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::{ASCII, UNICODE};

    /// Representación compacta para comparar barras en los tests.
    fn draw(cells: &[BarCell]) -> String {
        cells
            .iter()
            .map(|cell| match cell {
                BarCell::Filled => '=',
                BarCell::Head => '>',
                BarCell::Empty => '-',
                BarCell::Separator => '|',
                BarCell::More => '.',
                BarCell::Unknown => '~',
            })
            .collect()
    }

    fn bar(weights: &[u64], position: Option<u64>, width: usize, policy: BarPolicy) -> String {
        let input = BarInput {
            weights: weights.to_vec(),
            position,
        };
        draw(&segmented_bar(&input, width, policy, 1))
    }

    #[test]
    fn single_segment_at_start_middle_and_end() {
        assert_eq!(bar(&[100], Some(0), 10, SONG_BAR), ">---------");
        assert_eq!(bar(&[100], Some(32), 10, SONG_BAR), "===>------");
        assert_eq!(bar(&[100], Some(100), 10, SONG_BAR), "=========>");
    }

    #[test]
    fn stopped_bar_has_no_head() {
        assert_eq!(bar(&[100], None, 6, SONG_BAR), "------");
        assert_eq!(bar(&[1, 1], None, 5, ALBUM_BAR), "--|--");
    }

    #[test]
    fn segments_are_separated_and_filled_in_order() {
        assert_eq!(bar(&[10, 10, 10], Some(15), 11, ALBUM_BAR), "===|=>-|---");
    }

    #[test]
    fn largest_remainder_sums_exactly_the_width() {
        let sizes = allocate_cells(&[1, 1, 1], 10, 1);
        assert_eq!(sizes, [4, 3, 3]);
        let sizes = allocate_cells(&[5, 300, 7, 90], 37, 1);
        assert_eq!(sizes.iter().sum::<usize>(), 37);
        assert!(sizes.iter().all(|size| *size >= 1));
    }

    #[test]
    fn minimum_cells_take_from_the_largest_excess() {
        let sizes = allocate_cells(&[1, 1, 100], 10, 3);
        assert_eq!(sizes, [3, 3, 4]);
    }

    #[test]
    fn window_shrinks_until_segments_fit() {
        let weights = [10; 20];
        let drawn = bar(&weights, Some(105), 30, QUEUE_BAR);
        assert_eq!(drawn.chars().count(), 30);
        assert!(drawn.starts_with(".|") && drawn.ends_with("|."));
        assert_eq!(drawn.matches('>').count(), 1);
        let narrow = bar(&weights, Some(105), 12, QUEUE_BAR);
        assert_eq!(narrow, ".|====>---|.");
    }

    #[test]
    fn window_stays_whole_when_everything_fits() {
        assert_eq!(bar(&[10, 10], Some(0), 9, QUEUE_BAR), ">---|----");
    }

    #[test]
    fn wide_more_glyph_is_accounted_for() {
        let input = BarInput {
            weights: vec![10; 20],
            position: Some(105),
        };
        let cells = segmented_bar(&input, 30, QUEUE_BAR, 2);
        let columns: usize = cells
            .iter()
            .map(|cell| if *cell == BarCell::More { 2 } else { 1 })
            .sum();
        assert_eq!(columns, 30);
    }

    #[test]
    fn count_mode_is_just_equal_weights() {
        assert_eq!(bar(&[1, 1, 1, 1], Some(2), 7, ALBUM_BAR), "=|=|>|-");
    }

    #[test]
    fn meter_rounds_to_nearest_cell() {
        let filled = |cells: Vec<MeterCell>| {
            cells
                .iter()
                .filter(|cell| **cell == MeterCell::Filled)
                .count()
        };
        assert_eq!(filled(meter(70, 100, 10)), 7);
        assert_eq!(filled(meter(65, 100, 10)), 7);
        assert_eq!(filled(meter(0, 100, 10)), 0);
        assert_eq!(filled(meter(413, 469, 6)), 5);
        assert_eq!(meter(1, 0, 3).len(), 3);
    }

    #[test]
    fn lines_use_the_glyph_set() {
        let cells = [BarCell::Filled, BarCell::Head, BarCell::Empty];
        assert_eq!(bar_line(&cells, &ASCII).to_string(), "[=>-]");
        let unicode = format!(
            "{}{}{}",
            UNICODE.bar_filled, UNICODE.bar_head, UNICODE.bar_empty
        );
        assert_eq!(bar_line(&cells, &UNICODE).to_string(), unicode);
        assert_eq!(meter_width(10, &ASCII), 12);
    }

    #[test]
    fn unknown_bar_centers_its_label() {
        let line = unknown_bar_line(10, " x ", &ASCII);
        assert_eq!(line.to_string(), "[~~~ x ~~~~]");
        assert_eq!(unknown_bar_line(2, " en vivo ", &ASCII).to_string(), "[~~]");
    }
}
