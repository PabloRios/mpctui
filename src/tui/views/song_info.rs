//! Ficha del tema superpuesta y centrada: los tags (los conocidos traducidos, el resto
//! con su nombre) y, separados por una fila en blanco, los datos del archivo. Los
//! valores largos se parten en varias filas; si no entra, se desplaza.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::chrome::{Context, titled_block};
use super::format::{audio_parts, audio_text, format_duration, saturating_u16, wrap_text};
use crate::domain::{Song, SongDetails, TagName};
use crate::i18n::Phrase;
use crate::tui::state::SongInfoView;
use crate::tui::theme::Role;

/// Ancho máximo de la ficha, para que no se estire en pantallas anchas.
const MAX_WIDTH: usize = 90;
/// Columnas libres a cada lado de la ficha cuando ocupa toda la pantalla.
const SCREEN_MARGIN: usize = 2;
/// Bordes, fila en blanco arriba, fila en blanco y teclas abajo.
const FIXED_ROWS: usize = 5;
/// Columnas de los bordes y el margen interior.
const FIXED_COLUMNS: usize = 4;
/// Entre la etiqueta y el valor.
const LABEL_GAP: usize = 2;

/// Una fila lógica de la ficha, antes de partir el valor.
enum InfoRow<'a> {
    Field {
        label: &'a str,
        value: String,
    },
    /// Separa los tags de los datos del archivo.
    Gap,
}

pub fn render(frame: &mut Frame, area: Rect, info: &mut SongInfoView, context: &Context<'_>) {
    let SongInfoView {
        song,
        details,
        scroll,
    } = info;
    let rows = info_rows(song, details, context);
    let label_width = rows
        .iter()
        .map(|row| match row {
            InfoRow::Field { label, .. } => Span::raw(*label).width(),
            InfoRow::Gap => 0,
        })
        .max()
        .unwrap_or_default();
    let value_needed = rows
        .iter()
        .map(|row| match row {
            InfoRow::Field { value, .. } => Span::raw(value.as_str()).width(),
            InfoRow::Gap => 0,
        })
        .max()
        .unwrap_or_default();
    let available = usize::from(area.width)
        .saturating_sub(SCREEN_MARGIN * 2)
        .min(MAX_WIDTH);
    let width = (FIXED_COLUMNS + label_width + LABEL_GAP + value_needed).min(available);
    let value_width = width.saturating_sub(FIXED_COLUMNS + label_width + LABEL_GAP);
    let lines = info_lines(&rows, label_width, value_width, context);
    let visible = lines
        .len()
        .min(usize::from(area.height).saturating_sub(FIXED_ROWS));
    *scroll = (*scroll).min(lines.len().saturating_sub(visible));
    let mut shown = vec![Line::default()];
    shown.extend(lines.into_iter().skip(*scroll).take(visible));
    shown.push(Line::default());
    let keys = context.phrase(Phrase::SongInfoKeys);
    shown.push(Line::styled(format!("{keys} "), context.style(Role::Muted)).right_aligned());
    let popup = area.centered(
        Constraint::Length(saturating_u16(width)),
        Constraint::Length(saturating_u16(visible + FIXED_ROWS).min(area.height)),
    );
    frame.render_widget(Clear, popup);
    let title = context.phrase(Phrase::SongInfoTitle);
    // Es la ventana con foco: el borde va en `accent`.
    let title = Line::styled(title, context.style(Role::TextStrong));
    let block = titled_block(title, Role::Accent, context);
    frame.render_widget(Paragraph::new(shown).block(block), popup);
}

/// Los tags en su orden (sin repetir la etiqueta de un tag con varios valores) y los
/// datos del archivo que se conocen.
fn info_rows<'a>(song: &Song, details: &'a SongDetails, context: &Context<'_>) -> Vec<InfoRow<'a>> {
    let mut rows = Vec::new();
    let mut previous: Option<&TagName> = None;
    for tag in details.ordered_tags() {
        let label = if previous == Some(&tag.name) {
            ""
        } else {
            tag_label(&tag.name, context)
        };
        previous = Some(&tag.name);
        rows.push(InfoRow::Field {
            label,
            value: tag.value.clone(),
        });
    }
    if !rows.is_empty() {
        rows.push(InfoRow::Gap);
    }
    rows.extend(
        file_fields(song, details, context)
            .into_iter()
            .map(|(phrase, value)| InfoRow::Field {
                label: context.phrase(phrase),
                value,
            }),
    );
    rows
}

/// Archivo, duración, formato y fecha de modificación; lo que no se conoce se omite.
/// El códec sale de la extensión del tema.
fn file_fields(song: &Song, details: &SongDetails, context: &Context<'_>) -> Vec<(Phrase, String)> {
    let SongDetails {
        file,
        duration,
        format,
        last_modified,
        ..
    } = details;
    let audio = audio_text(&audio_parts(
        Some(song),
        format.as_ref(),
        None,
        context.texts,
    ));
    [
        (Phrase::InfoFile, Some(file.clone())),
        (Phrase::InfoDuration, duration.map(format_duration)),
        (
            Phrase::InfoFormat,
            Some(audio).filter(|text| !text.is_empty()),
        ),
        (Phrase::InfoLastModified, last_modified.clone()),
    ]
    .into_iter()
    .filter_map(|(phrase, value)| Some((phrase, value?)))
    .collect()
}

fn tag_label<'a>(name: &'a TagName, context: &Context<'_>) -> &'a str {
    let phrase = match name {
        TagName::Title => Phrase::TagTitle,
        TagName::Artist => Phrase::TagArtist,
        TagName::Album => Phrase::TagAlbum,
        TagName::AlbumArtist => Phrase::TagAlbumArtist,
        TagName::Date => Phrase::TagDate,
        TagName::OriginalDate => Phrase::TagOriginalDate,
        TagName::Track => Phrase::TagTrack,
        TagName::Disc => Phrase::TagDisc,
        TagName::Genre => Phrase::TagGenre,
        TagName::Composer => Phrase::TagComposer,
        TagName::Performer => Phrase::TagPerformer,
        TagName::Conductor => Phrase::TagConductor,
        TagName::Comment => Phrase::TagComment,
        TagName::Other(name) => return name,
    };
    context.phrase(phrase)
}

/// Cada valor partido en filas de `value_width`; la etiqueta solo en la primera.
fn info_lines<'a>(
    rows: &[InfoRow<'a>],
    label_width: usize,
    value_width: usize,
    context: &Context<'_>,
) -> Vec<Line<'a>> {
    let label_style = context.style(Role::Muted);
    let indent = " ".repeat(label_width + LABEL_GAP);
    let mut lines = Vec::new();
    for row in rows {
        let InfoRow::Field { label, value } = row else {
            lines.push(Line::default());
            continue;
        };
        for (index, part) in wrap_text(value, value_width).into_iter().enumerate() {
            let lead = if index == 0 {
                Span::styled(
                    format!("{label:<label_width$}{}", " ".repeat(LABEL_GAP)),
                    label_style,
                )
            } else {
                Span::raw(indent.clone())
            };
            lines.push(Line::from(vec![Span::raw(" "), lead, Span::raw(part)]));
        }
    }
    lines
}
