//! Diálogos superpuestos y centrados: confirmación, nombre de playlist (guardar la cola
//! o crear una nueva) o de partición, y selectores de playlist y de partición.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::chrome::{Context, titled_block};
use super::format::{fit_start, saturating_u16};
use super::theme;
use super::widgets::{ItemRows, RowSource, Selection, render_list};
use crate::domain::PartitionName;
use crate::i18n::{Message, Phrase};
use crate::tui::glyphs::Glyphs;
use crate::tui::list::SelectableList;
use crate::tui::picker::{Picker, PlaylistChoice};
use crate::tui::prompt::{Confirmation, DECLINE_KEYS, Prompt, PromptKind};
use crate::tui::state::AppState;

/// Bordes, filas en blanco y contenido: siempre cinco filas interiores.
const DIALOG_HEIGHT: u16 = 7;
const CONFIRM_MIN_WIDTH: u16 = 46;
const SAVE_WIDTH: u16 = 56;
/// Margen interior a cada lado del texto.
const PADDING: usize = 2;
/// Columnas de los dos bordes.
const BORDERS: u16 = 2;
/// Entre la etiqueta y el campo, y al cerrar el campo: `Nombre: [texto]`.
const FIELD_OPEN: &str = ": [";
const FIELD_CLOSE: &str = "]";
const ANSWERS_GAP: &str = "        ";
const PICKER_WIDTH: u16 = 44;
/// Opciones visibles del selector; con más, la lista se desplaza.
const PICKER_MAX_ROWS: usize = 10;
/// Bordes, fila en blanco y fila de teclas.
const PICKER_FIXED_ROWS: usize = 4;
/// Separación entre una insignia y el nombre.
const BADGE_GAP: &str = " ";

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, context: &Context<'_>) {
    let Some(prompt) = &state.prompt else {
        return;
    };
    match &prompt.kind {
        PromptKind::Search => {}
        PromptKind::SavePlaylist => {
            name_dialog(frame, area, prompt, Phrase::SaveQueueTitle, context);
        }
        PromptKind::NewPlaylist(_) => {
            name_dialog(frame, area, prompt, Phrase::NewPlaylistTitle, context);
        }
        PromptKind::NewPartition => {
            name_dialog(frame, area, prompt, Phrase::NewPartitionTitle, context);
        }
        PromptKind::Confirm(confirmation) => {
            let question = question(confirmation, state, context);
            confirm_dialog(frame, area, question, context);
        }
    }
}

fn question(confirmation: &Confirmation, state: &AppState, context: &Context<'_>) -> String {
    match confirmation {
        Confirmation::ClearQueue => {
            context.message(Message::ConfirmClearQueue(state.queue.items().len()))
        }
        Confirmation::RemovePlaylist(name) => {
            context.message(Message::ConfirmRemovePlaylist(name.as_str()))
        }
        Confirmation::RemovePartition(partition) => {
            context.message(Message::ConfirmRemovePartition {
                name: partition.name.as_str(),
                outputs: partition.outputs.len(),
            })
        }
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

fn confirm_dialog(frame: &mut Frame, area: Rect, question: String, context: &Context<'_>) {
    let question_width = saturating_u16(Span::raw(&question).width());
    let width = (question_width + saturating_u16(PADDING * 2) + BORDERS).max(CONFIRM_MIN_WIDTH);
    let answers = Line::from(vec![
        Span::styled(
            format!("[{}]", context.phrase(Phrase::ConfirmKey)),
            theme::ACCENT,
        ),
        Span::raw(format!(" {}", context.phrase(Phrase::Yes))),
        Span::raw(ANSWERS_GAP),
        Span::styled(format!("[{}]", DECLINE_KEYS[0]), theme::ACCENT),
        Span::raw(format!(" {}", context.phrase(Phrase::No))),
    ])
    .centered();
    let rows = [
        Line::default(),
        padded(vec![Span::raw(question)]),
        Line::default(),
        answers,
        Line::default(),
    ];
    let title = context.phrase(Phrase::ConfirmTitle);
    dialog(frame, area, width, title, rows, context.glyphs);
}

/// El campo muestra el final del texto si no entra; un error de validación aparece en
/// la última fila y el diálogo sigue abierto.
fn name_dialog(
    frame: &mut Frame,
    area: Rect,
    prompt: &Prompt,
    title: Phrase,
    context: &Context<'_>,
) {
    let glyphs = context.glyphs;
    let width = SAVE_WIDTH.min(area.width);
    let inner = usize::from(width.saturating_sub(BORDERS));
    let label = format!("{}{FIELD_OPEN}", context.phrase(Phrase::NameLabel));
    let fixed = PADDING * 2 + Span::raw(&label).width() + Span::raw(FIELD_CLOSE).width();
    let field_width = inner.saturating_sub(fixed);
    let typed = format!("{}{}", prompt.input, glyphs.input_cursor);
    let shown = fit_start(&typed, field_width, glyphs.ellipsis);
    let padding = " ".repeat(field_width.saturating_sub(Span::raw(&shown).width()));
    let field = padded(vec![
        Span::styled(label, theme::MUTED),
        Span::raw(shown),
        Span::raw(padding),
        Span::styled(FIELD_CLOSE, theme::MUTED),
    ]);
    let error = match &prompt.error {
        Some(error) => padded(vec![
            Span::styled(glyphs.message_error, theme::ERROR),
            Span::styled(
                format!(" {}", context.message(Message::Input(error))),
                theme::ERROR,
            ),
        ]),
        None => Line::default(),
    };
    let rows = [
        Line::default(),
        field,
        Line::default(),
        padded(vec![Span::styled(
            context.phrase(Phrase::SaveKeys),
            theme::MUTED,
        )]),
        error,
    ];
    dialog(frame, area, width, context.phrase(title), rows, glyphs);
}

pub fn render_picker(frame: &mut Frame, area: Rect, picker: &mut Picker, context: &Context<'_>) {
    let glyphs = context.glyphs;
    match picker {
        Picker::Playlist(picker) => {
            let new_playlist = context.phrase(Phrase::NewPlaylistChoice);
            picker_dialog(
                frame,
                area,
                Phrase::PickerTitle,
                &mut picker.choices,
                context,
                |choices| choice_rows(choices, new_playlist, glyphs),
            );
        }
        Picker::Partition(picker) => picker_dialog(
            frame,
            area,
            Phrase::PartitionPickerTitle,
            &mut picker.choices,
            context,
            partition_rows,
        ),
    }
}

/// Marco, lista desplazable de opciones y teclas al pie.
fn picker_dialog<'a, T, S: RowSource>(
    frame: &mut Frame,
    area: Rect,
    title: Phrase,
    choices: &'a mut SelectableList<T>,
    context: &Context<'_>,
    make_rows: impl FnOnce(&'a [T]) -> S,
) {
    let glyphs = context.glyphs;
    let visible = choices.items().len().min(PICKER_MAX_ROWS);
    let height = saturating_u16(visible + PICKER_FIXED_ROWS);
    let popup = area.centered(
        Constraint::Length(PICKER_WIDTH.min(area.width)),
        Constraint::Length(height.min(area.height)),
    );
    frame.render_widget(Clear, popup);
    let title = Line::styled(context.phrase(title).to_owned(), theme::BOLD);
    let block = titled_block(title, glyphs);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let [list_area, _, keys_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner.inner(Margin::new(1, 0)));
    let (items, table) = choices.parts_mut();
    let rows = make_rows(items);
    let selection = Selection {
        row: table.selected(),
        focused: true,
        centered: false,
    };
    let mut offset = table.offset();
    render_list(frame, list_area, glyphs, &rows, selection, &mut offset);
    *table.offset_mut() = offset;
    let keys = Line::styled(context.phrase(Phrase::PickerKeys).to_owned(), theme::MUTED);
    frame
        .buffer_mut()
        .set_line(keys_area.x, keys_area.y, &keys, keys_area.width);
}

fn partition_rows<'a>(names: &'a [PartitionName]) -> impl RowSource + 'a {
    ItemRows {
        items: names,
        line: |name: &'a PartitionName| Line::raw(name.as_str()),
    }
}

/// «Nueva playlist» resaltada arriba y las existentes con su insignia.
fn choice_rows<'a>(
    choices: &'a [PlaylistChoice],
    new_playlist: &'a str,
    glyphs: &'a Glyphs,
) -> impl RowSource + 'a {
    ItemRows {
        items: choices,
        line: move |choice: &'a PlaylistChoice| match choice {
            PlaylistChoice::New => Line::styled(new_playlist, theme::ACCENT),
            PlaylistChoice::Existing(name) => Line::from(vec![
                Span::styled(glyphs.playlist, theme::MUTED),
                Span::raw(BADGE_GAP),
                Span::raw(name.as_str()),
            ]),
        },
    }
}
