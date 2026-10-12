//! Diálogos superpuestos y centrados: confirmación y guardar la cola como playlist.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::chrome::{Context, titled_block};
use super::format::{fit_start, saturating_u16, songs_count};
use super::theme;
use crate::tui::glyphs::Glyphs;
use crate::tui::prompt::{CONFIRM_KEYS, Confirmation, DECLINE_KEYS, Prompt, PromptKind};
use crate::tui::state::AppState;

/// Bordes, filas en blanco y contenido: siempre cinco filas interiores.
const DIALOG_HEIGHT: u16 = 7;
const CONFIRM_MIN_WIDTH: u16 = 46;
const SAVE_WIDTH: u16 = 56;
/// Margen interior a cada lado del texto.
const PADDING: usize = 2;
/// Columnas de los dos bordes.
const BORDERS: u16 = 2;
const SAVE_TITLE: &str = "Guardar cola como playlist";
const CONFIRM_TITLE: &str = "Confirmar";
const NAME_LABEL: &str = "Nombre: [";
const FIELD_CLOSE: &str = "]";
const SAVE_KEYS: &str = "Enter guarda   Esc cancela";
const ANSWERS_GAP: &str = "        ";

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let Some(prompt) = &state.prompt else {
        return;
    };
    match &prompt.kind {
        PromptKind::Search => {}
        PromptKind::SavePlaylist => save_dialog(frame, area, prompt, context.glyphs),
        PromptKind::Confirm(confirmation) => {
            let question = question(confirmation, state);
            confirm_dialog(frame, area, question, context.glyphs);
        }
    }
}

fn question(confirmation: &Confirmation, state: &AppState) -> String {
    match confirmation {
        Confirmation::ClearQueue => format!(
            "¿Vaciar la cola? Se quitan {}.",
            songs_count(state.queue.items().len())
        ),
        Confirmation::RemovePlaylist(name) => format!("¿Borrar la playlist «{name}»?"),
    }
}

/// Dibuja el marco del diálogo y sus cinco filas interiores.
fn dialog(
    frame: &mut Frame,
    area: Rect,
    width: u16,
    title: &str,
    rows: [Line<'_>; 5],
    glyphs: &Glyphs,
) {
    let popup = area.centered(
        Constraint::Length(width.min(area.width)),
        Constraint::Length(DIALOG_HEIGHT.min(area.height)),
    );
    frame.render_widget(Clear, popup);
    let block = titled_block(Line::styled(title.to_owned(), theme::BOLD), glyphs);
    frame.render_widget(Paragraph::new(rows.to_vec()).block(block), popup);
}

fn padded<'a>(spans: Vec<Span<'a>>) -> Line<'a> {
    let mut line = vec![Span::raw(" ".repeat(PADDING))];
    line.extend(spans);
    Line::from(line)
}

fn confirm_dialog(frame: &mut Frame, area: Rect, question: String, glyphs: &Glyphs) {
    let question_width = saturating_u16(Span::raw(&question).width());
    let width = (question_width + saturating_u16(PADDING * 2) + BORDERS).max(CONFIRM_MIN_WIDTH);
    let answers = Line::from(vec![
        Span::styled(format!("[{}]", CONFIRM_KEYS[0]), theme::ACCENT),
        Span::raw(" sí"),
        Span::raw(ANSWERS_GAP),
        Span::styled(format!("[{}]", DECLINE_KEYS[0]), theme::ACCENT),
        Span::raw(" no"),
    ])
    .centered();
    let rows = [
        Line::default(),
        padded(vec![Span::raw(question)]),
        Line::default(),
        answers,
        Line::default(),
    ];
    dialog(frame, area, width, CONFIRM_TITLE, rows, glyphs);
}

/// El campo muestra el final del texto si no entra; un error de validación aparece en
/// la última fila y el diálogo sigue abierto.
fn save_dialog(frame: &mut Frame, area: Rect, prompt: &Prompt, glyphs: &Glyphs) {
    let width = SAVE_WIDTH.min(area.width);
    let inner = usize::from(width.saturating_sub(BORDERS));
    let fixed = PADDING * 2 + Span::raw(NAME_LABEL).width() + Span::raw(FIELD_CLOSE).width();
    let field_width = inner.saturating_sub(fixed);
    let typed = format!("{}{}", prompt.input, glyphs.input_cursor);
    let shown = fit_start(&typed, field_width, glyphs.ellipsis);
    let padding = " ".repeat(field_width.saturating_sub(Span::raw(&shown).width()));
    let field = padded(vec![
        Span::styled(NAME_LABEL, theme::MUTED),
        Span::raw(shown),
        Span::raw(padding),
        Span::styled(FIELD_CLOSE, theme::MUTED),
    ]);
    let error = match &prompt.error {
        Some(error) => padded(vec![
            Span::styled(glyphs.message_error, theme::ERROR),
            Span::styled(format!(" {error}"), theme::ERROR),
        ]),
        None => Line::default(),
    };
    let rows = [
        Line::default(),
        field,
        Line::default(),
        padded(vec![Span::styled(SAVE_KEYS, theme::MUTED)]),
        error,
    ];
    dialog(frame, area, width, SAVE_TITLE, rows, glyphs);
}
