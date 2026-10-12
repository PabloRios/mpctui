//! Correlación entre canales: `-1 ────┼────●──── +1   correlación +0.62`.

use ratatui::text::{Line, Span};

use crate::tui::glyphs::Glyphs;
use crate::tui::views::theme;

/// Celdas de la línea entre `-1` y `+1` (impar, para que haya centro).
const RULE_WIDTH: usize = 41;
const MINIMUM_LABEL: &str = "-1 ";
const MAXIMUM_LABEL: &str = " +1";
const VALUE_PREFIX: &str = "   correlación ";

/// La línea entra en `available` columnas: si no alcanza, se acorta la regla.
pub fn correlation_line(value: f32, available: u16, glyphs: &Glyphs) -> Line<'static> {
    let value_text = format!("{value:+.2}");
    let fixed = [MINIMUM_LABEL, MAXIMUM_LABEL, VALUE_PREFIX, &value_text]
        .iter()
        .map(|text| Span::raw(*text).width())
        .sum::<usize>();
    let width = RULE_WIDTH.min(usize::from(available).saturating_sub(fixed));
    let mut spans = vec![Span::styled(MINIMUM_LABEL, theme::MUTED)];
    spans.extend(rule(value, width, glyphs));
    spans.push(Span::styled(MAXIMUM_LABEL, theme::MUTED));
    spans.push(Span::styled(VALUE_PREFIX, theme::MUTED));
    spans.push(Span::styled(value_text, theme::BOLD));
    Line::from(spans)
}

/// La regla con el centro y el marcador del valor (que tapa al centro si coincide).
fn rule(value: f32, width: usize, glyphs: &Glyphs) -> Vec<Span<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let last = width - 1;
    let marker = ((value.clamp(-1.0, 1.0) + 1.0) / 2.0 * last as f32).round() as usize;
    (0..width)
        .map(|cell| {
            if cell == marker {
                Span::styled(glyphs.correlation_marker, theme::ACCENT)
            } else if cell == last / 2 {
                Span::styled(glyphs.correlation_center, theme::MUTED)
            } else {
                Span::styled(glyphs.border.horizontal_top, theme::MUTED)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::glyphs::ASCII;

    #[test]
    fn the_marker_sits_on_its_value() {
        let line = correlation_line(1.0, 80, &ASCII).to_string();
        assert!(line.starts_with("-1 ----"), "{line}");
        assert!(line.contains("-o +1   correlación +1.00"), "{line}");
        assert_eq!(line.matches('+').count(), 3);
        let centered = correlation_line(0.0, 80, &ASCII).to_string();
        assert!(
            centered.contains("-o-") && !centered.contains("-+-"),
            "{centered}"
        );
    }

    #[test]
    fn the_rule_shrinks_to_fit() {
        let line = correlation_line(-0.5, 40, &ASCII);
        assert!(line.width() <= 40);
        assert!(line.to_string().ends_with("correlación -0.50"));
    }
}
