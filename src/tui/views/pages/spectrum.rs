//! Vista Espectro: barras por banda alineadas abajo, picos, eje de frecuencias y
//! título. Si no se puede leer el FIFO, explica cómo configurarlo.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::{draw_line, text_row};
use crate::domain::{SPECTRUM_MAX_HZ, SPECTRUM_MIN_HZ, SpectrumFrame, band_centers};
use crate::service::SpectrumState;
use crate::tui::glyphs::Glyphs;
use crate::tui::state::{AppState, SpectrumView};
use crate::tui::views::chrome::Context;
use crate::tui::views::format::{frequency, frequency_tick};
use crate::tui::views::theme;
use crate::tui::views::widgets::{centered_message, spread};

pub const SPECTRUM_MIN_BANDS: usize = 8;
pub const SPECTRUM_MAX_BANDS: usize = 64;
const TITLE_ROWS: u16 = 1;
const AXIS_ROWS: u16 = 1;
/// Frecuencias del eje, en Hz.
const AXIS_FREQUENCIES: [u32; 9] = [63, 125, 250, 500, 1_000, 2_000, 4_000, 8_000, 16_000];
/// Tercios de la altura, para el color.
const HEIGHT_ZONES: usize = 3;
const NO_SIGNAL: &str = "sin señal";
const PART_GAP: &str = "  ";

/// Barras de 2 columnas con 1 de separación; en modo compacto, 1 columna sin separación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BarShape {
    width: u16,
    gap: u16,
}

const FULL_BARS: BarShape = BarShape { width: 2, gap: 1 };
const COMPACT_BARS: BarShape = BarShape { width: 1, gap: 0 };

impl BarShape {
    fn for_context(context: &Context<'_>) -> Self {
        if context.is_compact() {
            COMPACT_BARS
        } else {
            FULL_BARS
        }
    }

    /// Cuántas bandas entran en `width` columnas, entre el mínimo y el máximo.
    fn bands_in(self, width: u16) -> usize {
        let step = usize::from(self.width + self.gap);
        let bands = (usize::from(width) + usize::from(self.gap)) / step;
        bands.clamp(SPECTRUM_MIN_BANDS, SPECTRUM_MAX_BANDS)
    }

    fn total_width(self, bands: usize) -> usize {
        let step = usize::from(self.width + self.gap);
        (bands * step).saturating_sub(usize::from(self.gap))
    }
}

/// Bandas que muestra la vista en un área de `width` columnas (sin el margen).
pub fn band_count(width: u16, compact: bool) -> usize {
    let shape = if compact { COMPACT_BARS } else { FULL_BARS };
    shape.bands_in(text_width(width))
}

/// Ancho útil: el área menos el margen de los costados.
fn text_width(width: u16) -> u16 {
    text_row(Rect::new(0, 0, width, 1)).width
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let view = &state.spectrum;
    if context.is_compact() {
        return match &view.state {
            SpectrumState::Frame(spectrum) => draw_bars(frame, text_row(area), spectrum, context),
            SpectrumState::Unavailable(reason) => draw_help(frame, area, view, reason),
        };
    }
    let [title, body] =
        Layout::vertical([Constraint::Length(TITLE_ROWS), Constraint::Fill(1)]).areas(area);
    draw_title(frame, text_row(title), view, context);
    match &view.state {
        SpectrumState::Frame(spectrum) => {
            let [bars, axis] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(AXIS_ROWS)])
                    .areas(text_row(body));
            draw_bars(frame, bars, spectrum, context);
            draw_axis(frame, axis, spectrum.levels.len(), context);
        }
        SpectrumState::Unavailable(reason) => draw_help(frame, body, view, reason),
    }
}

/// `Espectro` a la izquierda; a la derecha `32 bandas  40 Hz - 16 kHz  /tmp/mpd.fifo`,
/// o `sin señal` si no se puede leer.
fn draw_title(frame: &mut Frame, area: Rect, view: &SpectrumView, context: &Context<'_>) {
    let details = match &view.state {
        SpectrumState::Frame(spectrum) => [
            format!("{} bandas", spectrum.levels.len()),
            format!(
                "{} - {}",
                frequency(SPECTRUM_MIN_HZ),
                frequency(SPECTRUM_MAX_HZ)
            ),
            view.source.clone(),
        ]
        .join(PART_GAP),
        SpectrumState::Unavailable(_) => NO_SIGNAL.to_owned(),
    };
    let line = spread(
        Line::styled("Espectro", theme::BOLD),
        Line::styled(details, theme::MUTED),
        area.width,
        context.glyphs,
    );
    draw_line(frame, area, &line);
}

/// Columna donde empieza la primera barra, para centrar el conjunto.
fn first_column(area: Rect, shape: BarShape, bands: usize) -> u16 {
    let spare = usize::from(area.width).saturating_sub(shape.total_width(bands));
    area.x + u16::try_from(spare / 2).unwrap_or_default()
}

/// Bandas que entran en el área (el cuadro puede ser de otro ancho tras un cambio de
/// tamaño, hasta que llegue el próximo).
fn visible_bands(area: Rect, shape: BarShape, bands: usize) -> usize {
    let step = usize::from(shape.width + shape.gap);
    bands.min((usize::from(area.width) + usize::from(shape.gap)) / step)
}

fn draw_bars(frame: &mut Frame, area: Rect, spectrum: &SpectrumFrame, context: &Context<'_>) {
    if area.is_empty() {
        return;
    }
    if !spectrum.signal && spectrum.is_flat() {
        return centered_message(frame, area, Line::styled(NO_SIGNAL, theme::MUTED));
    }
    let shape = BarShape::for_context(context);
    let bands = visible_bands(area, shape, spectrum.levels.len());
    let start = first_column(area, shape, bands);
    let rows = usize::from(area.height);
    for band in 0..bands {
        let column = BarColumn {
            level: spectrum.levels[band],
            peak: spectrum.peaks.get(band).copied().unwrap_or_default(),
            rows,
        };
        let x = start + u16::try_from(band).unwrap_or_default() * (shape.width + shape.gap);
        for row in 0..rows {
            let Some((symbol, style)) = column.cell(row, context.glyphs) else {
                continue;
            };
            let y = area.bottom() - 1 - u16::try_from(row).unwrap_or_default();
            let text = symbol.repeat(usize::from(shape.width));
            frame.buffer_mut().set_string(x, y, text, style);
        }
    }
}

/// Una barra en sub-celdas: `rows × niveles por celda` de alto.
struct BarColumn {
    level: f32,
    peak: f32,
    rows: usize,
}

impl BarColumn {
    /// Glifo y estilo de la fila `row` contando desde abajo; `None` si va vacía.
    fn cell(&self, row: usize, glyphs: &Glyphs) -> Option<(&'static str, Style)> {
        let steps = glyphs.spectrum_levels.len() - 1;
        let filled = sub_cells(self.level, self.rows, steps);
        let fill = filled.saturating_sub(row * steps).min(steps);
        let style = zone_style(row, self.rows);
        if fill > 0 {
            return Some((glyphs.spectrum_levels[fill], style));
        }
        let peak = sub_cells(self.peak, self.rows, steps);
        (peak > 0 && peak / steps == row).then_some((glyphs.spectrum_peak, style))
    }
}

/// Altura de `level` en sub-celdas.
fn sub_cells(level: f32, rows: usize, steps: usize) -> usize {
    (level.clamp(0.0, 1.0) * (rows * steps) as f32).round() as usize
}

/// Verde el tercio de abajo, amarillo el del medio y rojo el de arriba.
fn zone_style(row: usize, rows: usize) -> Style {
    match (row * HEIGHT_ZONES) / rows.max(1) {
        0 => theme::SPECTRUM_LOW,
        1 => theme::SPECTRUM_MIDDLE,
        _ => theme::SPECTRUM_HIGH,
    }
}

/// Etiquetas debajo de la banda más cercana a cada frecuencia. Si una se pisa con la
/// anterior, se omite.
fn draw_axis(frame: &mut Frame, area: Rect, bands: usize, context: &Context<'_>) {
    let shape = BarShape::for_context(context);
    let bands = visible_bands(area, shape, bands);
    if bands == 0 {
        return;
    }
    let start = first_column(area, shape, bands);
    let centers = band_centers(bands);
    let mut free_from = area.x;
    for hertz in AXIS_FREQUENCIES {
        let label = frequency_tick(hertz);
        let width = u16::try_from(Span::raw(&label).width()).unwrap_or_default();
        let band = nearest_band(&centers, hertz as f32);
        let bar = start + u16::try_from(band).unwrap_or_default() * (shape.width + shape.gap);
        let x = (bar + shape.width / 2)
            .saturating_sub(width / 2)
            .min(area.right().saturating_sub(width));
        if x < free_from {
            continue;
        }
        frame
            .buffer_mut()
            .set_string(x, area.y, label, theme::MUTED);
        free_from = x + width + 1;
    }
}

/// La banda cuyo centro está más cerca de `hertz`, en escala logarítmica.
fn nearest_band(centers: &[f32], hertz: f32) -> usize {
    let distance = |center: &f32| (center.ln() - hertz.ln()).abs();
    centers
        .iter()
        .enumerate()
        .min_by(|first, second| distance(first.1).total_cmp(&distance(second.1)))
        .map(|(band, _)| band)
        .unwrap_or_default()
}

/// Qué falta y cómo configurarlo.
fn draw_help(frame: &mut Frame, area: Rect, view: &SpectrumView, reason: &str) {
    let source = &view.source;
    let lines = [
        String::new(),
        format!("  No se puede leer {source} ({reason})."),
        String::new(),
        "  Agregá una salida fifo en mpd.conf y reiniciá MPD:".to_owned(),
        String::new(),
        "      audio_output {".to_owned(),
        "          type   \"fifo\"".to_owned(),
        "          name   \"visualizador\"".to_owned(),
        format!("          path   \"{source}\""),
        format!("          format \"{}\"", view.format),
        "      }".to_owned(),
        String::new(),
        "  Si usás otra ruta o formato, configurá visualizer_fifo y visualizer_format.".to_owned(),
    ];
    let rows = Layout::vertical(vec![Constraint::Length(1); lines.len()]).split(text_row(area));
    for (text, row) in lines.into_iter().zip(rows.iter()) {
        draw_line(frame, *row, &Line::raw(text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::{ASCII, UNICODE};

    #[test]
    fn band_count_follows_the_width() {
        assert_eq!(band_count(98, false), 32);
        assert_eq!(band_count(10, false), SPECTRUM_MIN_BANDS);
        assert_eq!(band_count(400, false), SPECTRUM_MAX_BANDS);
        assert_eq!(band_count(70, true), SPECTRUM_MAX_BANDS);
        assert_eq!(band_count(40, true), 38);
    }

    #[test]
    fn bars_use_partial_cells_at_the_top() {
        // 2 filas × 8 niveles: 0,5 llena la fila de abajo justo.
        let half = BarColumn {
            level: 0.5,
            peak: 0.5,
            rows: 2,
        };
        assert_eq!(half.cell(0, &UNICODE).unwrap().0, "█");
        assert_eq!(half.cell(1, &UNICODE).unwrap().0, "▔");
        let partial = BarColumn {
            level: 0.75,
            peak: 0.75,
            rows: 2,
        };
        assert_eq!(partial.cell(1, &UNICODE).unwrap().0, "▄");
        assert_eq!(partial.cell(1, &ASCII).unwrap().0, ":");
    }

    #[test]
    fn peaks_only_show_on_empty_cells() {
        let column = BarColumn {
            level: 0.2,
            peak: 0.9,
            rows: 4,
        };
        let cells: Vec<_> = (0..4)
            .map(|row| column.cell(row, &ASCII).map(|(symbol, _)| symbol))
            .collect();
        assert_eq!(cells, [Some(":"), None, None, Some("-")]);
        let empty = BarColumn {
            level: 0.0,
            peak: 0.0,
            rows: 4,
        };
        assert!((0..4).all(|row| empty.cell(row, &ASCII).is_none()));
    }

    #[test]
    fn colors_go_by_thirds() {
        assert_eq!(zone_style(0, 9), theme::SPECTRUM_LOW);
        assert_eq!(zone_style(4, 9), theme::SPECTRUM_MIDDLE);
        assert_eq!(zone_style(8, 9), theme::SPECTRUM_HIGH);
    }

    #[test]
    fn axis_labels_go_under_the_nearest_band() {
        let centers = band_centers(32);
        let band = nearest_band(&centers, 1_000.0);
        let edges = crate::domain::band_edges(32);
        assert!(edges[band] <= 1_000.0 * 1.2 && 1_000.0 / 1.2 < edges[band + 1]);
        assert_eq!(nearest_band(&centers, 20_000.0), 31);
    }
}
