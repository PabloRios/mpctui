//! Correlación entre canales: `-1 ────┼────●──── +1   correlación +0.62`.

use ratatui::text::{Line, Span};

use crate::i18n::Phrase;
use crate::tui::theme::Role;
use crate::tui::views::chrome::Context;

/// Celdas de la línea entre `-1` y `+1` (impar, para que haya centro).
const RULE_WIDTH: usize = 41;
const MINIMUM_LABEL: &str = "-1 ";
const MAXIMUM_LABEL: &str = " +1";
/// Antes y después del nombre del valor.
const VALUE_GAP: &str = "   ";
/// Hasta esta correlación la imagen es muy abierta: el marcador avisa.
const CORRELATION_WIDE: f32 = 0.3;

/// La línea entra en `available` columnas: si no alcanza, se acorta la regla.
pub fn correlation_line(value: f32, available: u16, context: &Context<'_>) -> Line<'static> {
    let value_text = format!("{value:+.2}");
    let prefix = format!("{VALUE_GAP}{} ", context.phrase(Phrase::Correlation));
    let fixed = [MINIMUM_LABEL, MAXIMUM_LABEL, &prefix, &value_text]
        .iter()
        .map(|text| Span::raw(*text).width())
        .sum::<usize>();
    let width = RULE_WIDTH.min(usize::from(available).saturating_sub(fixed));
    let muted = context.style(Role::Muted);
    let mut spans = vec![Span::styled(MINIMUM_LABEL, muted)];
    spans.extend(rule(value, width, context));
    spans.push(Span::styled(MAXIMUM_LABEL, muted));
    spans.push(Span::styled(prefix, muted));
    spans.push(Span::styled(value_text, context.style(Role::TextStrong)));
    Line::from(spans)
}

/// Negativa: problemas de fase. Hasta `CORRELATION_WIDE`: muy abierta. Arriba, sana.
fn marker_role(value: f32) -> Role {
    if value < 0.0 {
        Role::Error
    } else if value < CORRELATION_WIDE {
        Role::Warning
    } else {
        Role::Success
    }
}

/// La regla con el centro y el marcador del valor (que tapa al centro si coincide).
fn rule(value: f32, width: usize, context: &Context<'_>) -> Vec<Span<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let glyphs = context.glyphs;
    let line = context.style(Role::Subtle);
    let last = width - 1;
    let marker = ((value.clamp(-1.0, 1.0) + 1.0) / 2.0 * last as f32).round() as usize;
    (0..width)
        .map(|cell| {
            if cell == marker {
                Span::styled(glyphs.correlation_marker, context.style(marker_role(value)))
            } else if cell == last / 2 {
                Span::styled(glyphs.correlation_center, line)
            } else {
                Span::styled(glyphs.border.horizontal_top, line)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::English;
    use crate::tui::glyphs::ASCII;
    use crate::tui::views::chrome::testing;

    #[test]
    fn the_marker_sits_on_its_value() {
        let context = testing::context(&ASCII);
        let line = correlation_line(1.0, 80, &context).to_string();
        assert!(line.starts_with("-1 ----"), "{line}");
        assert!(line.contains("-o +1   correlación +1.00"), "{line}");
        assert_eq!(line.matches('+').count(), 3);
        let centered = correlation_line(0.0, 80, &context).to_string();
        assert!(
            centered.contains("-o-") && !centered.contains("-+-"),
            "{centered}"
        );
    }

    #[test]
    fn the_rule_shrinks_to_fit() {
        let context = testing::context(&ASCII);
        let line = correlation_line(-0.5, 40, &context);
        assert!(line.width() <= 40);
        assert!(line.to_string().ends_with("correlación -0.50"));
        let english = Context {
            texts: &English,
            ..context
        };
        let line = correlation_line(-0.5, 40, &english);
        assert!(line.width() <= 40);
        assert!(line.to_string().ends_with("correlation -0.50"));
    }

    #[test]
    fn the_marker_warns_about_phase_and_width() {
        assert_eq!(marker_role(-0.2), Role::Error);
        assert_eq!(marker_role(0.1), Role::Warning);
        assert_eq!(marker_role(CORRELATION_WIDE), Role::Success);
        assert_eq!(marker_role(1.0), Role::Success);
    }
}
