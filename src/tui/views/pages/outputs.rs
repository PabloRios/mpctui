//! Salidas (`8`): particiones a la izquierda y salidas de la elegida a la derecha.
//! La columna con foco lleva el título entre corchetes y la selección invertida.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};

use super::{draw_line, draw_list};
use crate::domain::{Output, Partition, PartitionName};
use crate::i18n::{Message, Phrase};
use crate::tui::action::Action;
use crate::tui::state::{AppState, OutputsColumn};
use crate::tui::theme::Role;
use crate::tui::views::chrome::{self, Context};
use crate::tui::views::widgets::{ItemRows, RowSource, centered_message, panel_title};

const PARTITIONS_SHARE: u16 = 30;
/// Separación entre paneles: la ocupa el divisor.
const DIVIDER_WIDTH: u16 = 1;
/// Separación entre el indicador y el nombre, y entre el nombre y su detalle.
const BADGE_GAP: &str = " ";
const DETAIL_GAP: &str = "  ";

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, context: &Context<'_>) {
    let view = &mut state.outputs;
    let [titles_area, rule_area, lists_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [partitions_column, outputs_column] = Layout::horizontal([
        Constraint::Percentage(PARTITIONS_SHARE),
        Constraint::Fill(1),
    ])
    .spacing(DIVIDER_WIDTH)
    .areas(area);
    chrome::rule(frame.buffer_mut(), rule_area, context, None);
    let partitions_focused = view.focus == OutputsColumn::Partitions;
    let partitions_title = panel_title(
        format!(
            "{} ({})",
            context.phrase(Phrase::PartitionsTitle),
            view.partitions.items().len()
        ),
        partitions_focused,
        context,
    );
    draw_line(
        frame,
        column_row(partitions_column, titles_area),
        &Line::from(partitions_title),
    );
    let outputs_title = outputs_title(view.selected_partition(), !partitions_focused, context);
    draw_line(
        frame,
        column_row(outputs_column, titles_area),
        &outputs_title,
    );

    let in_use = &state.partition;
    let rows_context = *context;
    draw_list(
        frame,
        column_row(partitions_column, lists_area),
        context,
        &mut view.partitions,
        partitions_focused,
        |partitions| partition_rows(partitions, in_use, rows_context),
    );
    let outputs_area = column_row(outputs_column, lists_area);
    if view.outputs.items().is_empty() {
        let key = context.key(Action::MoveOutput);
        let text = context.message(Message::NoOutputs { key: &key });
        let text = Line::styled(text, context.style(Role::Muted));
        centered_message(frame, outputs_area, text);
    } else {
        draw_list(
            frame,
            outputs_area,
            context,
            &mut view.outputs,
            !partitions_focused,
            |outputs| output_rows(outputs, context),
        );
    }
    let divider_rows = area.y..area.bottom();
    chrome::divider(
        frame.buffer_mut(),
        partitions_column.right(),
        divider_rows,
        context,
        true,
    );
}

/// La parte de una columna que cae en una franja horizontal.
fn column_row(column: Rect, band: Rect) -> Rect {
    Rect::new(column.x, band.y, column.width, band.height)
}

/// `cocina: 2 salidas`.
fn outputs_title(
    partition: Option<&Partition>,
    focused: bool,
    context: &Context<'_>,
) -> Line<'static> {
    let Some(partition) = partition else {
        return Line::default();
    };
    let count = context.message(Message::OutputCount(partition.outputs.len()));
    let title = format!("{}: {count}", partition.name);
    Line::from(panel_title(title, focused, context))
}

/// `◆ cocina  2 salidas`: la marca solo en la que usa mpctui; las demás, alineadas.
fn partition_rows<'a>(
    partitions: &'a [Partition],
    in_use: &'a PartitionName,
    context: Context<'a>,
) -> impl RowSource + 'a {
    let glyphs = context.glyphs;
    let blank = " ".repeat(Span::raw(glyphs.partition_in_use).width());
    ItemRows {
        items: partitions,
        line: move |partition: &'a Partition| {
            let count = context.message(Message::OutputCount(partition.outputs.len()));
            let marker = if partition.name == *in_use {
                Span::styled(glyphs.partition_in_use, context.style(Role::Accent))
            } else {
                Span::raw(blank.clone())
            };
            Line::from(vec![
                marker,
                Span::raw(BADGE_GAP),
                Span::raw(partition.name.as_str()),
                Span::styled(format!("{DETAIL_GAP}{count}"), context.style(Role::Muted)),
            ])
        },
    }
}

/// `● pulse audio  pulse`: el indicador es el mismo de los modos del panel.
fn output_rows<'a>(outputs: &'a [Output], context: &Context<'_>) -> impl RowSource + use<'a> {
    let glyphs = context.glyphs;
    let (open, close) = (glyphs.bar_open, glyphs.bar_close);
    let active = (glyphs.flag_on, context.style(Role::Success));
    let inactive = (glyphs.flag_off, context.style(Role::Subtle));
    let muted = context.style(Role::Muted);
    ItemRows {
        items: outputs,
        line: move |output: &'a Output| {
            let (flag, style) = if output.enabled { active } else { inactive };
            Line::from(vec![
                Span::styled(format!("{open}{flag}{close}"), style),
                Span::raw(BADGE_GAP),
                Span::raw(output.name.as_str()),
                Span::styled(format!("{DETAIL_GAP}{}", output.plugin), muted),
            ])
        },
    }
}
