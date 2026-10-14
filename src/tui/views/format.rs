//! Formatos de texto para mostrar datos del dominio.

use std::time::Duration;

use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use crate::domain::{AudioFormat, STEREO_CHANNELS, SampleFormat, Song, Tag, VisualizerStyle};
use crate::i18n::{Catalog, Message, Phrase};

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const MILLIS_PER_SECOND: u128 = 1_000;
/// Texto para un dato que falta.
pub const MISSING: &str = "—";
/// Tiempo que no se conoce (detenido o stream).
pub const TIME_UNKNOWN: &str = "--:--";
/// Cifras del año al principio de `Date`.
const YEAR_DIGITS: usize = 4;
/// Separación entre las partes del formato de audio.
pub const AUDIO_PART_SEPARATOR: &str = "  ";
/// Las URL (`http://…`) son streams: su "extensión" no dice nada del códec.
const URL_SCHEME_SEPARATOR: &str = "://";
const EXTENSION_SEPARATOR: char = '.';
const HERTZ_PER_KILOHERTZ: u32 = 1_000;
/// Cifras de la parte decimal de los kHz (`44.1`, `22.05`).
const KILOHERTZ_DECIMALS: usize = 3;
const MONO_CHANNELS: u8 = 1;
/// Códecs sin pérdida y con pérdida, según la extensión. `M4A` puede ser AAC o ALAC:
/// queda como desconocido.
const LOSSLESS_CODECS: [&str; 8] = ["FLAC", "ALAC", "WAV", "AIFF", "APE", "WV", "DSF", "DFF"];
const LOSSY_CODECS: [&str; 6] = ["MP3", "OGG", "OPUS", "AAC", "MPC", "WMA"];
/// Alta resolución: más que esta frecuencia, o al menos estos bits.
const HIGH_RESOLUTION_ABOVE_HERTZ: u32 = 48_000;
const HIGH_RESOLUTION_MIN_BITS: u8 = 24;

/// `m:ss`, o `h:mm:ss` si dura una hora o más.
pub fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let hours = total / SECONDS_PER_HOUR;
    let minutes = total % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
    let seconds = total % SECONDS_PER_MINUTE;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Segundos que faltan, redondeados hacia arriba y nunca `0 s` (`5 s`); desde un minuto,
/// `m:ss`.
pub fn countdown(remaining: Duration) -> String {
    let rounded_up = remaining.as_millis().div_ceil(MILLIS_PER_SECOND).max(1);
    let seconds = u64::try_from(rounded_up).unwrap_or(u64::MAX);
    if seconds < SECONDS_PER_MINUTE {
        format!("{seconds} s")
    } else {
        format_duration(Duration::from_secs(seconds))
    }
}

pub fn duration_or_unknown(duration: Option<Duration>) -> String {
    duration.map_or_else(|| TIME_UNKNOWN.to_owned(), format_duration)
}

/// El año, si `Date` empieza con cuatro dígitos; si no, nada (es un tag libre).
pub fn year(date: Option<&str>) -> Option<&str> {
    let year = date?.get(..YEAR_DIGITS)?;
    year.bytes()
        .all(|byte| byte.is_ascii_digit())
        .then_some(year)
}

/// `Artista - Álbum (año)`, o `[sin álbum]`.
pub fn album_title(song: &Song) -> AlbumTitle<'_> {
    AlbumTitle {
        artist: song.album.as_ref().and(song.album_owner()),
        album: song.album.as_deref(),
        year: song.album.as_ref().and(year(song.date.as_deref())),
    }
}

/// Las partes del título de un tramo, para colorear cada una: `Artista - Álbum (año)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlbumTitle<'a> {
    pub artist: Option<&'a str>,
    /// `None`: temas sin álbum (`[sin álbum]`).
    pub album: Option<&'a str>,
    pub year: Option<&'a str>,
}

/// Entre el artista y el álbum.
pub const ARTIST_ALBUM_SEPARATOR: &str = " - ";

pub fn tag_label(tag: Tag) -> Phrase {
    match tag {
        Tag::Any => Phrase::TagAny,
        Tag::Artist => Phrase::TagArtist,
        Tag::AlbumArtist => Phrase::TagAlbumArtist,
        Tag::Album => Phrase::TagAlbum,
        Tag::Title => Phrase::TagTitle,
    }
}

pub fn or_missing(value: Option<&str>) -> &str {
    value.unwrap_or(MISSING)
}

pub fn or_unnamed<'a>(value: &'a str, texts: &dyn Catalog) -> &'a str {
    if value.is_empty() {
        texts.phrase(Phrase::Unnamed)
    } else {
        value
    }
}

/// Códec según la extensión del archivo, en mayúsculas (`FLAC`). Sin extensión o en
/// streams, nada.
pub fn codec(song: &Song) -> Option<String> {
    if song.file.contains(URL_SCHEME_SEPARATOR) {
        return None;
    }
    let (_, extension) = song.file_name().rsplit_once(EXTENSION_SEPARATOR)?;
    (!extension.is_empty()).then(|| extension.to_uppercase())
}

/// Hz a kHz sin ceros de más: `44.1 kHz`, `48 kHz`.
pub fn sample_rate(hertz: u32) -> String {
    let whole = hertz / HERTZ_PER_KILOHERTZ;
    let fraction = hertz % HERTZ_PER_KILOHERTZ;
    if fraction == 0 {
        return format!("{whole} kHz");
    }
    let decimals = format!("{fraction:0KILOHERTZ_DECIMALS$}");
    format!("{whole}.{} kHz", decimals.trim_end_matches('0'))
}

/// `40 Hz`, `16 kHz`.
pub fn frequency(hertz: f32) -> String {
    let rounded = hertz.round() as u32;
    if rounded < HERTZ_PER_KILOHERTZ {
        format!("{rounded} Hz")
    } else {
        sample_rate(rounded)
    }
}

/// Etiqueta del eje del espectro: `63`, `500`, `1k`, `16k`.
pub fn frequency_tick(hertz: u32) -> String {
    if hertz < HERTZ_PER_KILOHERTZ {
        hertz.to_string()
    } else {
        format!("{}k", hertz / HERTZ_PER_KILOHERTZ)
    }
}

/// Nombre de cada forma del visualizador.
pub fn visualizer_style_name(style: VisualizerStyle) -> Phrase {
    match style {
        VisualizerStyle::Bars => Phrase::StyleBars,
        VisualizerStyle::Mirror => Phrase::StyleMirror,
        VisualizerStyle::Wave => Phrase::StyleWave,
        VisualizerStyle::Waterfall => Phrase::StyleWaterfall,
        VisualizerStyle::Vectorscope => Phrase::StyleVectorscope,
        VisualizerStyle::Meters => Phrase::StyleMeters,
    }
}

/// `mono`, `estéreo` o `n canales`.
pub fn channels(count: u8, texts: &dyn Catalog) -> String {
    match count {
        MONO_CHANNELS => texts.phrase(Phrase::Mono).to_owned(),
        STEREO_CHANNELS => texts.phrase(Phrase::Stereo).to_owned(),
        _ => texts.message(&Message::ChannelCount(count)),
    }
}

/// Calidad del códec, para colorearlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioQuality {
    Lossless,
    Lossy,
    Unknown,
}

/// Por la extensión, en mayúsculas.
pub fn codec_quality(codec: &str) -> AudioQuality {
    if LOSSLESS_CODECS.contains(&codec) {
        AudioQuality::Lossless
    } else if LOSSY_CODECS.contains(&codec) {
        AudioQuality::Lossy
    } else {
        AudioQuality::Unknown
    }
}

/// Alta resolución exige un códec sin pérdida: MPD decodifica MP3 a 24 bit y, si no,
/// todo MP3 lo sería.
pub fn is_high_resolution(quality: AudioQuality, audio: &AudioFormat) -> bool {
    let rich = match audio.format {
        SampleFormat::Integer { bits } => bits >= HIGH_RESOLUTION_MIN_BITS,
        SampleFormat::Float | SampleFormat::Dsd { .. } => true,
    };
    let fast = audio
        .sample_rate
        .is_some_and(|hertz| hertz > HIGH_RESOLUTION_ABOVE_HERTZ);
    quality == AudioQuality::Lossless && (rich || fast)
}

/// Qué dice cada parte del formato de audio, para colorearla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPartKind {
    Codec(AudioQuality),
    /// Frecuencia y profundidad (o la tasa DSD).
    Sample {
        high_resolution: bool,
    },
    Channels,
    Bitrate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioPart {
    pub text: String,
    pub kind: AudioPartKind,
}

/// Frecuencia y profundidad; en DSD, solo la tasa (`DSD64`).
fn sample_parts(audio: &AudioFormat, texts: &dyn Catalog) -> Vec<String> {
    let rate = audio.sample_rate.map(sample_rate);
    let depth = match audio.format {
        SampleFormat::Integer { bits } => Some(format!("{bits} bit")),
        SampleFormat::Float => Some(texts.phrase(Phrase::FloatSamples).to_owned()),
        SampleFormat::Dsd { rate } => return vec![format!("DSD{rate}")],
    };
    rate.into_iter().chain(depth).collect()
}

/// `FLAC  44.1 kHz  16 bit  estéreo  1411 kbps`, en partes; lo que no se conoce se omite.
pub fn audio_parts(
    song: Option<&Song>,
    audio: Option<&AudioFormat>,
    bitrate: Option<u32>,
    texts: &dyn Catalog,
) -> Vec<AudioPart> {
    let codec = song.and_then(codec);
    let quality = codec
        .as_deref()
        .map_or(AudioQuality::Unknown, codec_quality);
    let part = |text: String, kind| AudioPart { text, kind };
    let mut parts: Vec<AudioPart> = codec
        .into_iter()
        .map(|codec| part(codec, AudioPartKind::Codec(quality)))
        .collect();
    if let Some(audio) = audio {
        let high_resolution = is_high_resolution(quality, audio);
        let sample = AudioPartKind::Sample { high_resolution };
        parts.extend(
            sample_parts(audio, texts)
                .into_iter()
                .map(|text| part(text, sample)),
        );
        parts.push(part(
            channels(audio.channels, texts),
            AudioPartKind::Channels,
        ));
    }
    parts.extend(bitrate.map(|kbps| part(format!("{kbps} kbps"), AudioPartKind::Bitrate)));
    parts
}

/// Las partes unidas con su separación.
pub fn audio_text(parts: &[AudioPart]) -> String {
    parts
        .iter()
        .map(|part| part.text.as_str())
        .collect::<Vec<_>>()
        .join(AUDIO_PART_SEPARATOR)
}

/// Las partes que entran en `width`, quitando desde la izquierda: nunca corta una parte
/// por la mitad. Si no entra ninguna, `None`.
pub fn fit_parts_from_left(parts: &[AudioPart], width: usize) -> Option<&[AudioPart]> {
    (0..parts.len())
        .map(|start| &parts[start..])
        .find(|fitting| Span::raw(audio_text(fitting)).width() <= width)
}

/// Las cifras en `number` y el resto en `base`: `cola: 1229 temas  65:04:13`.
pub fn numbers_stand_out(text: &str, base: Style, number: Style) -> Line<'static> {
    let spans: Vec<Span<'static>> = text
        .chars()
        .collect::<Vec<_>>()
        .chunk_by(|left, right| left.is_ascii_digit() == right.is_ascii_digit())
        .map(|run| {
            let style = if run[0].is_ascii_digit() {
                number
            } else {
                base
            };
            Span::styled(run.iter().collect::<String>(), style)
        })
        .collect();
    Line::from(spans)
}

/// Convierte una cantidad de filas o columnas al tipo de ratatui, sin desbordar.
pub fn saturating_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Recorta la línea para que entre en `width` columnas visibles, terminándola con
/// `ellipsis` (con el estilo del último tramo que quedó). Mide anchos, nunca bytes.
pub fn fit_line<'a>(line: Line<'a>, width: usize, ellipsis: &'a str) -> Line<'a> {
    if line.width() <= width {
        return line;
    }
    let Line {
        style,
        alignment,
        spans,
    } = line;
    let mut budget = width.saturating_sub(Span::raw(ellipsis).width());
    let mut kept = Vec::new();
    let mut ellipsis_style = style;
    for span in spans {
        ellipsis_style = span.style;
        let span_width = span.width();
        if span_width > budget {
            let prefix = prefix_within(&span.content, budget);
            if !prefix.is_empty() {
                kept.push(Span::styled(prefix.to_owned(), span.style));
            }
            break;
        }
        budget -= span_width;
        kept.push(span);
    }
    kept.push(Span::styled(ellipsis, ellipsis_style));
    Line {
        style,
        alignment,
        spans: kept,
    }
}

/// Recorta por la izquierda: lo importante es el final (dónde estoy, lo último tipeado).
pub fn fit_start(text: &str, width: usize, ellipsis: &str) -> String {
    if Span::raw(text).width() <= width {
        return text.to_owned();
    }
    let budget = width.saturating_sub(Span::raw(ellipsis).width());
    format!("{ellipsis}{}", suffix_within(text, budget))
}

/// Como `fit_start`, para una línea con estilos: se recorta por la izquierda y la
/// elipsis toma el estilo de lo que corta.
pub fn fit_line_start<'a>(line: Line<'a>, width: usize, ellipsis: &'a str) -> Line<'a> {
    if line.width() <= width {
        return line;
    }
    let mut budget = width.saturating_sub(Span::raw(ellipsis).width());
    let mut kept = Vec::new();
    let mut ellipsis_style = line.style;
    for span in line.spans.into_iter().rev() {
        ellipsis_style = span.style;
        let span_width = span.width();
        if span_width > budget {
            let suffix = suffix_within(&span.content, budget);
            if !suffix.is_empty() {
                kept.push(Span::styled(suffix.to_owned(), span.style));
            }
            break;
        }
        budget -= span_width;
        kept.push(span);
    }
    kept.push(Span::styled(ellipsis, ellipsis_style));
    kept.reverse();
    Line::from(kept)
}

/// El final más largo de `text` que entra en `width` columnas.
fn suffix_within(text: &str, width: usize) -> &str {
    let mut used = 0;
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, character)| {
            used += character.width().unwrap_or_default();
            used <= width
        })
        .last()
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

/// Celda de ancho fijo: recorta con `ellipsis` y rellena con espacios según la alineación.
pub fn cell<'a>(
    line: Line<'a>,
    width: usize,
    alignment: Alignment,
    ellipsis: &'a str,
) -> Vec<Span<'a>> {
    let fitted = fit_line(line, width, ellipsis);
    let padding = " ".repeat(width.saturating_sub(fitted.width()));
    let mut spans = fitted.spans;
    match alignment {
        Alignment::Right => spans.insert(0, Span::raw(padding)),
        Alignment::Left | Alignment::Center => spans.push(Span::raw(padding)),
    }
    spans
}

/// Parte el texto en tramos, resaltando lo que coincide con la búsqueda (sin
/// distinguir mayúsculas).
pub fn highlight_matches<'a>(
    text: &'a str,
    query: Option<&str>,
    style: Style,
    match_style: Style,
) -> Line<'a> {
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return Line::from(Span::styled(text, style));
    };
    let mut spans = Vec::new();
    let mut plain_from = 0;
    let mut index = 0;
    while index < text.len() {
        match match_length(&text[index..], query) {
            Some(length) => {
                if plain_from < index {
                    spans.push(Span::styled(&text[plain_from..index], style));
                }
                spans.push(Span::styled(
                    &text[index..index + length],
                    style.patch(match_style),
                ));
                index += length;
                plain_from = index;
            }
            None => index += text[index..].chars().next().map_or(1, char::len_utf8),
        }
    }
    if plain_from < text.len() {
        spans.push(Span::styled(&text[plain_from..], style));
    }
    Line::from(spans)
}

/// Bytes de `text` que coinciden con `query` al principio, sin distinguir mayúsculas.
fn match_length(text: &str, query: &str) -> Option<usize> {
    let mut text_characters = text.char_indices();
    for query_character in query.chars() {
        let (_, text_character) = text_characters.next()?;
        if !text_character
            .to_lowercase()
            .eq(query_character.to_lowercase())
        {
            return None;
        }
    }
    Some(
        text_characters
            .next()
            .map_or(text.len(), |(index, _)| index),
    )
}

/// El prefijo más largo de `text` que ocupa como mucho `width` columnas.
fn prefix_within(text: &str, width: usize) -> &str {
    let mut used = 0;
    for (index, character) in text.char_indices() {
        used += character.width().unwrap_or_default();
        if used > width {
            return &text[..index];
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    use ratatui::style::{Color, Style};

    use crate::i18n::{English, Spanish};
    use crate::tui::glyphs::{ASCII, UNICODE};

    fn fitted(text: &str, width: usize, ellipsis: &'static str) -> String {
        fit_line(Line::from(text.to_owned()), width, ellipsis).to_string()
    }

    #[test]
    fn durations_use_minutes_or_hours() {
        assert_eq!(format_duration(Duration::from_secs(5)), "0:05");
        assert_eq!(format_duration(Duration::from_secs(208)), "3:28");
        assert_eq!(format_duration(Duration::from_secs(3_725)), "1:02:05");
    }

    #[test]
    fn countdown_rounds_up_and_switches_to_minutes() {
        assert_eq!(countdown(Duration::from_millis(300)), "1 s");
        assert_eq!(countdown(Duration::ZERO), "1 s");
        assert_eq!(countdown(Duration::from_millis(4_200)), "5 s");
        assert_eq!(countdown(Duration::from_secs(59)), "59 s");
        assert_eq!(countdown(Duration::from_secs(75)), "1:15");
    }

    #[test]
    fn empty_values_have_placeholders() {
        assert_eq!(or_missing(None), MISSING);
        assert_eq!(or_unnamed("", &Spanish), "(sin nombre)");
        assert_eq!(or_unnamed("", &English), "(unnamed)");
        assert_eq!(or_unnamed("Rock", &English), "Rock");
    }

    #[test]
    fn unknown_times_and_plurals() {
        assert_eq!(duration_or_unknown(None), TIME_UNKNOWN);
    }

    #[test]
    fn year_needs_four_leading_digits() {
        assert_eq!(year(Some("1973-03-01")), Some("1973"));
        assert_eq!(year(Some("1973")), Some("1973"));
        assert_eq!(year(Some("circa 1970")), None);
        assert_eq!(year(Some("97")), None);
        assert_eq!(year(None), None);
    }

    #[test]
    fn album_title_uses_album_artist_and_year() {
        let mut song = Song {
            artist: Some("Uno".into()),
            album: Some("Éxitos".into()),
            date: Some("1983".into()),
            ..Song::default()
        };
        let title = |artist, album, year| AlbumTitle {
            artist,
            album,
            year,
        };
        assert_eq!(
            album_title(&song),
            title(Some("Uno"), Some("Éxitos"), Some("1983"))
        );
        song.album_artist = Some("Varios".into());
        song.date = None;
        assert_eq!(
            album_title(&song),
            title(Some("Varios"), Some("Éxitos"), None)
        );
        song.album = None;
        song.date = Some("1983".into());
        assert_eq!(album_title(&song), title(None, None, None));
    }

    #[test]
    fn fit_start_keeps_the_end() {
        let path = "/ > Rock > Pink Floyd > Meddle";
        assert_eq!(fit_start(path, 40, ASCII.ellipsis), path);
        assert_eq!(fit_start(path, 12, ASCII.ellipsis), "... > Meddle");
        let expected = format!("{}ción", UNICODE.ellipsis);
        assert_eq!(fit_start("Canción", 5, UNICODE.ellipsis), expected);
    }

    #[test]
    fn fit_line_start_keeps_the_end_and_its_styles() {
        let last = Style::new().fg(Color::White);
        let line = Line::from(vec![Span::raw("/ > Rock > "), Span::styled("Meddle", last)]);
        let fitted = fit_line_start(line.clone(), 12, ASCII.ellipsis);
        assert_eq!(fitted.to_string(), "... > Meddle");
        assert_eq!(fitted.spans.last().unwrap().style, last);
        assert_eq!(fit_line_start(line, 40, ASCII.ellipsis).width(), 17);
    }

    #[test]
    fn cells_pad_to_their_width() {
        let right = cell(Line::from("3:05"), 6, Alignment::Right, ASCII.ellipsis);
        assert_eq!(Line::from(right).to_string(), "  3:05");
        let left = cell(
            Line::from("Tiempo largo"),
            8,
            Alignment::Left,
            ASCII.ellipsis,
        );
        assert_eq!(Line::from(left).to_string(), "Tiemp...");
    }

    #[test]
    fn matches_are_highlighted_ignoring_case() {
        let found = Style::new().fg(Color::Yellow);
        let line = highlight_matches("Time After TIME", Some("time"), Style::new(), found);
        let highlighted: Vec<_> = line
            .spans
            .iter()
            .filter(|span| span.style == found)
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(highlighted, ["Time", "TIME"]);
        assert_eq!(line.to_string(), "Time After TIME");
        let accented = highlight_matches("Canción", Some("CIÓN"), Style::new(), found);
        assert_eq!(accented.spans.len(), 2);
        assert_eq!(
            highlight_matches("x", None, Style::new(), found)
                .spans
                .len(),
            1
        );
    }

    #[test]
    fn numbers_stand_out_from_the_text() {
        let number = Style::new().fg(Color::White);
        let line = numbers_stand_out("cola: 12 temas  3:05", Style::new(), number);
        let numbers: Vec<_> = line
            .spans
            .iter()
            .filter(|span| span.style == number)
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(numbers, ["12", "3", "05"]);
        assert_eq!(line.to_string(), "cola: 12 temas  3:05");
    }

    #[test]
    fn audio_quality_depends_on_the_codec() {
        assert_eq!(codec_quality("FLAC"), AudioQuality::Lossless);
        assert_eq!(codec_quality("MP3"), AudioQuality::Lossy);
        assert_eq!(codec_quality("M4A"), AudioQuality::Unknown);
        let mp3 = pcm(44_100, SampleFormat::Integer { bits: 24 }, 2);
        assert!(!is_high_resolution(AudioQuality::Lossy, &mp3));
        let studio = pcm(96_000, SampleFormat::Integer { bits: 24 }, 2);
        assert!(is_high_resolution(AudioQuality::Lossless, &studio));
        let compact_disc = pcm(44_100, SampleFormat::Integer { bits: 16 }, 2);
        assert!(!is_high_resolution(AudioQuality::Lossless, &compact_disc));
        let dsd = AudioFormat {
            sample_rate: None,
            format: SampleFormat::Dsd { rate: 64 },
            channels: 2,
        };
        assert!(is_high_resolution(AudioQuality::Lossless, &dsd));
        assert!(!is_high_resolution(AudioQuality::Unknown, &studio));
    }

    #[test]
    fn audio_parts_carry_their_kind() {
        let parts = audio_parts(
            Some(&song_file("a.mp3")),
            Some(&pcm(44_100, SampleFormat::Integer { bits: 24 }, 2)),
            Some(320),
            &Spanish,
        );
        let kinds: Vec<_> = parts.iter().map(|part| part.kind).collect();
        let sample = AudioPartKind::Sample {
            high_resolution: false,
        };
        assert_eq!(
            kinds,
            [
                AudioPartKind::Codec(AudioQuality::Lossy),
                sample,
                sample,
                AudioPartKind::Channels,
                AudioPartKind::Bitrate,
            ]
        );
    }

    fn song_file(file: &str) -> Song {
        Song {
            file: file.into(),
            ..Song::default()
        }
    }

    fn pcm(sample_rate: u32, format: SampleFormat, channels: u8) -> AudioFormat {
        AudioFormat {
            sample_rate: Some(sample_rate),
            format,
            channels,
        }
    }

    #[test]
    fn codec_comes_from_the_extension() {
        let codec_of = |file: &str| codec(&song_file(file));
        assert_eq!(codec_of("Rock/01 - Time.flac").as_deref(), Some("FLAC"));
        assert_eq!(codec_of("a.b/tema.Opus").as_deref(), Some("OPUS"));
        assert_eq!(codec_of("Rock/sin extension"), None);
        assert_eq!(codec_of("Rock/punto final."), None);
        assert_eq!(codec_of("http://radio.example/stream.mp3"), None);
    }

    #[test]
    fn sample_rates_drop_trailing_zeros() {
        assert_eq!(sample_rate(44_100), "44.1 kHz");
        assert_eq!(sample_rate(48_000), "48 kHz");
        assert_eq!(sample_rate(96_000), "96 kHz");
        assert_eq!(sample_rate(22_050), "22.05 kHz");
        assert_eq!(channels(1, &Spanish), "mono");
        assert_eq!(frequency(40.0), "40 Hz");
        assert_eq!(frequency(16_000.0), "16 kHz");
        assert_eq!(frequency_tick(63), "63");
        assert_eq!(frequency_tick(16_000), "16k");
        assert_eq!(channels(6, &Spanish), "6 canales");
        assert_eq!(channels(2, &English), "stereo");
        assert_eq!(channels(6, &English), "6 channels");
    }

    #[test]
    fn audio_parts_follow_the_table() {
        let joined = |file: &str, audio: Option<AudioFormat>, bitrate| {
            audio_text(&audio_parts(
                Some(&song_file(file)),
                audio.as_ref(),
                bitrate,
                &Spanish,
            ))
        };
        let integer = |bits| SampleFormat::Integer { bits };
        let cases = [
            (
                joined("a.flac", Some(pcm(44_100, integer(16), 2)), Some(1411)),
                "FLAC  44.1 kHz  16 bit  estéreo  1411 kbps",
            ),
            (
                joined("a.flac", Some(pcm(96_000, integer(24), 2)), Some(2741)),
                "FLAC  96 kHz  24 bit  estéreo  2741 kbps",
            ),
            (
                joined(
                    "a.dsf",
                    Some(AudioFormat {
                        sample_rate: None,
                        format: SampleFormat::Dsd { rate: 64 },
                        channels: 2,
                    }),
                    None,
                ),
                "DSF  DSD64  estéreo",
            ),
            (
                joined(
                    "a.opus",
                    Some(pcm(48_000, SampleFormat::Float, 2)),
                    Some(128),
                ),
                "OPUS  48 kHz  flotante  estéreo  128 kbps",
            ),
            (
                joined("http://radio/", Some(pcm(48_000, integer(16), 2)), None),
                "48 kHz  16 bit  estéreo",
            ),
            (joined("a.mp3", None, None), "MP3"),
        ];
        for (got, expected) in cases {
            assert_eq!(got, expected);
        }
        assert!(audio_parts(None, None, None, &English).is_empty());
    }

    #[test]
    fn audio_parts_are_dropped_from_the_left() {
        let parts: Vec<AudioPart> = ["FLAC", "44.1 kHz", "16 bit", "estéreo", "1411 kbps"]
            .map(|text| AudioPart {
                text: text.to_owned(),
                kind: AudioPartKind::Channels,
            })
            .into();
        let fitted = |width| fit_parts_from_left(&parts, width).map(audio_text);
        let full = "FLAC  44.1 kHz  16 bit  estéreo  1411 kbps";
        assert_eq!(fitted(60).as_deref(), Some(full));
        assert_eq!(
            fitted(full.chars().count() - 1).as_deref(),
            Some("44.1 kHz  16 bit  estéreo  1411 kbps")
        );
        assert_eq!(fitted(9).as_deref(), Some("1411 kbps"));
        assert_eq!(fitted(8), None);
        assert_eq!(fit_parts_from_left(&[], 10), None);
    }

    #[test]
    fn fit_line_keeps_lines_that_fit() {
        assert_eq!(fitted("Pink Floyd", 10, UNICODE.ellipsis), "Pink Floyd");
    }

    #[test]
    fn fit_line_measures_columns_not_bytes() {
        let expected = format!("Canción {}", UNICODE.ellipsis);
        assert_eq!(fitted("Canción del adiós", 9, UNICODE.ellipsis), expected);
        assert_eq!(fitted("Canción del adiós", 9, ASCII.ellipsis), "Canció...");
        assert_eq!(
            fitted("The Piper at the Gates", 8, ASCII.ellipsis),
            "The P..."
        );
    }

    #[test]
    fn fit_line_cuts_across_spans_and_keeps_styles() {
        let marker = Style::new().fg(Color::Cyan);
        let icon = format!("{} ", UNICODE.directory);
        let line = Line::from(vec![
            Span::styled(icon.clone(), marker),
            Span::raw("Directorio largo"),
        ]);
        let fitted = fit_line(line, 8, UNICODE.ellipsis);
        assert_eq!(
            fitted.to_string(),
            format!("{icon}Direc{}", UNICODE.ellipsis)
        );
        assert_eq!(fitted.spans[0].style, marker);
        assert_eq!(fitted.width(), 8);
    }
}
