//! Diálogos superpuestos y centrados: confirmación, nombre de playlist (guardar la cola
//! o crear una nueva) o de partición, y selectores de playlist, de partición y de orden.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::chrome::{Context, titled_block};
use super::format::{fit_start, saturating_u16};
use super::widgets::{ItemRows, RowSource, Selection, render_list};
use crate::domain::{PartitionName, SortKey};
use crate::i18n::{Message, Phrase};
use crate::tui::list::SelectableList;
use crate::tui::picker::{Picker, PlaylistChoice, SortOrder};
use crate::tui::prompt::{Confirmation, DECLINE_KEYS, Prompt, PromptKind};
use crate::tui::state::AppState;
use crate::tui::theme::Role;

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
/// Lo que ocupa el número de un criterio de orden sin elegir.
const UNRANKED: &str = " ";

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

/// Cómo se ve el marco de un diálogo: las confirmaciones (siempre de acciones
/// destructivas) avisan; el texto libre lleva el foco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogKind {
    Destructive,
    Input,
}

impl DialogKind {
    fn border(self) -> Role {
        match self {
            DialogKind::Destructive => Role::Warning,
            DialogKind::Input => Role::Accent,
        }
    }

    fn title(self, context: &Context<'_>) -> ratatui::style::Style {
        match self {
            DialogKind::Destructive => context.strong(Role::Warning),
            DialogKind::Input => context.style(Role::TextStrong),
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
    kind: DialogKind,
    context: &Context<'_>,
) {
    let popup = area.centered(
        Constraint::Length(width.min(area.width)),
        Constraint::Length(DIALOG_HEIGHT.min(area.height)),
    );
    frame.render_widget(Clear, popup);
    let title = Line::styled(title.to_owned(), kind.title(context));
    let block = titled_block(title, kind.border(), context);
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
    let key = context.strong(Role::Accent);
    let answers = Line::from(vec![
        Span::styled(format!("[{}]", context.phrase(Phrase::ConfirmKey)), key),
        Span::raw(format!(" {}", context.phrase(Phrase::Yes))),
        Span::raw(ANSWERS_GAP),
        Span::styled(format!("[{}]", DECLINE_KEYS[0]), key),
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
    dialog(
        frame,
        area,
        width,
        title,
        rows,
        DialogKind::Destructive,
        context,
    );
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
    let muted = context.style(Role::Muted);
    let field = padded(vec![
        Span::styled(label, muted),
        Span::raw(shown),
        Span::raw(padding),
        Span::styled(FIELD_CLOSE, muted),
    ]);
    let error = match &prompt.error {
        Some(error) => {
            let style = context.style(Role::Error);
            padded(vec![
                Span::styled(glyphs.message_error, style),
                Span::styled(
                    format!(" {}", context.message(Message::Input(error))),
                    style,
                ),
            ])
        }
        None => Line::default(),
    };
    let rows = [
        Line::default(),
        field,
        Line::default(),
        padded(vec![Span::styled(context.phrase(Phrase::SaveKeys), muted)]),
        error,
    ];
    let title = context.phrase(title);
    dialog(frame, area, width, title, rows, DialogKind::Input, context);
}

pub fn render_picker(frame: &mut Frame, area: Rect, picker: &mut Picker, context: &Context<'_>) {
    match picker {
        Picker::Playlist(picker) => {
            let new_playlist = context.phrase(Phrase::NewPlaylistChoice);
            picker_dialog(
                frame,
                area,
                [Phrase::PickerTitle, Phrase::PickerKeys],
                &mut picker.choices,
                context,
                |choices| choice_rows(choices, new_playlist, context),
            );
        }
        Picker::Partition(picker) => picker_dialog(
            frame,
            area,
            [Phrase::PartitionPickerTitle, Phrase::PickerKeys],
            &mut picker.choices,
            context,
            partition_rows,
        ),
        Picker::Sort(picker) => picker_dialog(
            frame,
            area,
            [Phrase::SortPickerTitle, Phrase::SortPickerKeys],
            &mut picker.choices,
            context,
            |keys| sort_rows(keys, &picker.order, context),
        ),
    }
}

/// Marco, lista desplazable de opciones y teclas al pie; `[título, teclas]`.
fn picker_dialog<'a, T, S: RowSource>(
    frame: &mut Frame,
    area: Rect,
    [title, keys]: [Phrase; 2],
    choices: &'a mut SelectableList<T>,
    context: &Context<'_>,
    make_rows: impl FnOnce(&'a [T]) -> S,
) {
    let visible = choices.items().len().min(PICKER_MAX_ROWS);
    let height = saturating_u16(visible + PICKER_FIXED_ROWS);
    let popup = area.centered(
        Constraint::Length(PICKER_WIDTH.min(area.width)),
        Constraint::Length(height.min(area.height)),
    );
    frame.render_widget(Clear, popup);
    let title = Line::styled(
        context.phrase(title).to_owned(),
        DialogKind::Input.title(context),
    );
    let block = titled_block(title, DialogKind::Input.border(), context);
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
    render_list(frame, list_area, context, &rows, selection, &mut offset);
    *table.offset_mut() = offset;
    let keys = Line::styled(context.phrase(keys).to_owned(), context.style(Role::Muted));
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

/// Cada criterio con su lugar en el orden; los elegidos, resaltados.
fn sort_rows<'a, 'texts>(
    keys: &'a [SortKey],
    order: &'a SortOrder,
    context: &Context<'texts>,
) -> impl RowSource + use<'a, 'texts> {
    let texts = context.texts;
    let chosen = context.style(Role::Accent);
    ItemRows {
        items: keys,
        line: move |key: &'a SortKey| {
            let name = texts.phrase(sort_key_phrase(*key));
            match order.rank(*key) {
                Some(rank) => Line::from(vec![
                    Span::styled(rank.to_string(), chosen),
                    Span::raw(BADGE_GAP),
                    Span::styled(name, chosen),
                ]),
                None => Line::from(vec![
                    Span::raw(UNRANKED),
                    Span::raw(BADGE_GAP),
                    Span::raw(name),
                ]),
            }
        },
    }
}

fn sort_key_phrase(key: SortKey) -> Phrase {
    match key {
        SortKey::Artist => Phrase::SortArtist,
        SortKey::Year => Phrase::SortYear,
        SortKey::Album => Phrase::SortAlbum,
        SortKey::Disc => Phrase::SortDisc,
        SortKey::Track => Phrase::SortTrack,
        SortKey::Title => Phrase::SortTitle,
        SortKey::File => Phrase::SortFile,
    }
}

/// «Nueva playlist» resaltada arriba y las existentes con su insignia.
fn choice_rows<'a>(
    choices: &'a [PlaylistChoice],
    new_playlist: &'a str,
    context: &Context<'_>,
) -> impl RowSource + use<'a> {
    let glyph = context.glyphs.playlist;
    let new_style = context.style(Role::Accent);
    let playlist = context.style(Role::Playlist);
    ItemRows {
        items: choices,
        line: move |choice: &'a PlaylistChoice| match choice {
            PlaylistChoice::New => Line::styled(new_playlist, new_style),
            PlaylistChoice::Existing(name) => Line::from(vec![
                Span::styled(glyph, playlist),
                Span::raw(BADGE_GAP),
                Span::styled(name.as_str(), playlist),
            ]),
        },
    }
}
