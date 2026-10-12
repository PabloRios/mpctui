//! Formato LRC: `[mm:ss.xx]texto`. Una línea puede tener varias marcas (estribillos
//! repetidos), hay metadatos (`[ar:…]`, `[offset:+250]`) y, en el formato extendido,
//! marcas por palabra (`<mm:ss.xx>`) que se descartan. Sin ninguna marca de tiempo, el
//! texto se toma como letra sin sincronizar.

use std::time::Duration;

use crate::domain::{LyricLine, Lyrics, LyricsSource};

const TAG_OPEN: char = '[';
const TAG_CLOSE: char = ']';
const WORD_TAG_OPEN: char = '<';
const WORD_TAG_CLOSE: char = '>';
const TIME_SEPARATOR: char = ':';
const FRACTION_SEPARATORS: [char; 2] = ['.', ':'];
/// Metadato que corre todas las marcas: positivo adelanta la letra.
const OFFSET_TAG: &str = "offset";
const MILLIS_DIGITS: usize = 3;
const SECONDS_PER_MINUTE: u64 = 60;
const MILLIS_PER_SECOND: u64 = 1_000;

/// Las marcas al principio de una línea y el texto que queda.
#[derive(Debug, Default)]
struct ParsedLine<'a> {
    times: Vec<Duration>,
    offset: Option<i64>,
    /// Tenía alguna marca que no es de tiempo (metadato).
    metadata: bool,
    text: &'a str,
}

pub fn parse_lrc(text: &str, source: LyricsSource) -> Lyrics {
    let parsed: Vec<ParsedLine<'_>> = text.lines().map(parse_line).collect();
    let offset = parsed
        .iter()
        .find_map(|line| line.offset)
        .unwrap_or_default();
    let synced = parsed.iter().any(|line| !line.times.is_empty());
    let lines = if synced {
        synced_lines(&parsed, offset)
    } else {
        plain_lines(&parsed)
    };
    Lyrics { lines, source }
}

/// Una línea por marca, corrida por el `offset` y ordenada por tiempo.
fn synced_lines(parsed: &[ParsedLine<'_>], offset: i64) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = parsed
        .iter()
        .flat_map(|line| {
            line.times.iter().map(|time| LyricLine {
                time: Some(shifted(*time, offset)),
                text: without_word_tags(line.text),
            })
        })
        .collect();
    lines.sort_by_key(|line| line.time);
    lines
}

/// El texto tal cual, sin metadatos ni líneas vacías en los bordes.
fn plain_lines(parsed: &[ParsedLine<'_>]) -> Vec<LyricLine> {
    let texts: Vec<&str> = parsed
        .iter()
        .filter(|line| !line.metadata)
        .map(|line| line.text.trim_end())
        .collect();
    let first = texts.iter().position(|text| !text.is_empty());
    let last = texts.iter().rposition(|text| !text.is_empty());
    let (Some(first), Some(last)) = (first, last) else {
        return Vec::new();
    };
    texts[first..=last]
        .iter()
        .map(|text| LyricLine {
            time: None,
            text: (*text).to_owned(),
        })
        .collect()
}

fn parse_line(line: &str) -> ParsedLine<'_> {
    let mut parsed = ParsedLine {
        text: line,
        ..ParsedLine::default()
    };
    let mut rest = line.trim_start();
    while let Some(after_open) = rest.strip_prefix(TAG_OPEN) {
        let Some((tag, after_tag)) = after_open.split_once(TAG_CLOSE) else {
            break;
        };
        if let Some(time) = parse_time(tag) {
            parsed.times.push(time);
        } else if let Some((key, value)) = tag.split_once(TIME_SEPARATOR) {
            parsed.metadata = true;
            if key.trim().eq_ignore_ascii_case(OFFSET_TAG) {
                parsed.offset = value.trim().parse().ok();
            }
        } else {
            break;
        }
        rest = after_tag;
    }
    if !parsed.times.is_empty() || parsed.metadata {
        parsed.text = rest.trim();
    }
    parsed
}

/// `mm:ss`, `mm:ss.x`, `mm:ss.xx` o `mm:ss.xxx` (también `mm:ss:xx`).
fn parse_time(tag: &str) -> Option<Duration> {
    let (minutes, rest) = tag.split_once(TIME_SEPARATOR)?;
    let (seconds, fraction) = match rest.split_once(FRACTION_SEPARATORS) {
        Some((seconds, fraction)) => (seconds, fraction),
        None => (rest, ""),
    };
    let digits = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    if !digits(minutes) || !digits(seconds) || !(fraction.is_empty() || digits(fraction)) {
        return None;
    }
    let minutes: u64 = minutes.parse().ok()?;
    let seconds: u64 = seconds.parse().ok()?;
    let millis: u64 = format!("{fraction:0<MILLIS_DIGITS$}")[..MILLIS_DIGITS]
        .parse()
        .ok()?;
    Some(Duration::from_millis(
        (minutes * SECONDS_PER_MINUTE + seconds) * MILLIS_PER_SECOND + millis,
    ))
}

/// Un `offset` positivo hace que la letra aparezca antes.
fn shifted(time: Duration, offset_millis: i64) -> Duration {
    let offset = Duration::from_millis(offset_millis.unsigned_abs());
    if offset_millis >= 0 {
        time.saturating_sub(offset)
    } else {
        time + offset
    }
}

/// Saca las marcas por palabra del LRC extendido: `<00:01.20>Hola <00:01.80>mundo`.
fn without_word_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(WORD_TAG_OPEN) {
        let after = &rest[start + WORD_TAG_OPEN.len_utf8()..];
        match after.split_once(WORD_TAG_CLOSE) {
            Some((tag, tail)) if parse_time(tag).is_some() => {
                result.push_str(&rest[..start]);
                rest = tail;
            }
            _ => {
                result.push_str(&rest[..=start]);
                rest = after;
            }
        }
    }
    result.push_str(rest);
    result.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times(lyrics: &Lyrics) -> Vec<u64> {
        lyrics
            .lines
            .iter()
            .map(|line| u64::try_from(line.time.unwrap().as_millis()).unwrap())
            .collect()
    }

    fn texts(lyrics: &Lyrics) -> Vec<&str> {
        lyrics.lines.iter().map(|line| line.text.as_str()).collect()
    }

    #[test]
    fn timestamps_accept_every_precision() {
        assert_eq!(parse_time("01:02"), Some(Duration::from_millis(62_000)));
        assert_eq!(parse_time("01:02.5"), Some(Duration::from_millis(62_500)));
        assert_eq!(parse_time("01:02.34"), Some(Duration::from_millis(62_340)));
        assert_eq!(parse_time("01:02.345"), Some(Duration::from_millis(62_345)));
        assert_eq!(parse_time("01:02:34"), Some(Duration::from_millis(62_340)));
        assert_eq!(parse_time("ar:Pink Floyd"), None);
        assert_eq!(parse_time("1:x"), None);
    }

    #[test]
    fn synced_lyrics_are_sorted_and_repeated_lines_expanded() {
        let text = "[ar:Pink Floyd]\n[ti:Time]\n\
                    [00:10.00][00:30.00]Estribillo\n\
                    [00:20.50]Verso\n\
                    [00:40.00]\n\
                    texto suelto";
        let lyrics = parse_lrc(text, LyricsSource::Cache);
        assert!(lyrics.is_synced());
        assert_eq!(times(&lyrics), [10_000, 20_500, 30_000, 40_000]);
        assert_eq!(texts(&lyrics), ["Estribillo", "Verso", "Estribillo", ""]);
        assert_eq!(lyrics.source, LyricsSource::Cache);
    }

    #[test]
    fn the_offset_shifts_every_line() {
        let earlier = parse_lrc(
            "[offset:+500]\n[00:01.00]a\n[00:00.20]b",
            LyricsSource::Cache,
        );
        assert_eq!(times(&earlier), [0, 500]);
        let later = parse_lrc("[offset:-500]\n[00:01.00]a", LyricsSource::Cache);
        assert_eq!(times(&later), [1_500]);
    }

    #[test]
    fn word_tags_are_removed() {
        let lyrics = parse_lrc(
            "[00:01.00]<00:01.00>Hola <00:01.50>mundo <3",
            LyricsSource::Tags,
        );
        assert_eq!(texts(&lyrics), ["Hola mundo <3"]);
    }

    #[test]
    fn text_without_timestamps_is_plain() {
        let lyrics = parse_lrc(
            "\n[ar:Alguien]\nPrimera\n\n  Segunda  \n\n",
            LyricsSource::Tags,
        );
        assert!(!lyrics.is_synced());
        assert_eq!(texts(&lyrics), ["Primera", "", "  Segunda"]);
        assert!(lyrics.lines.iter().all(|line| line.time.is_none()));
        assert!(parse_lrc("\n\n", LyricsSource::Tags).lines.is_empty());
    }

    #[test]
    fn brackets_in_plain_text_are_kept() {
        let lyrics = parse_lrc("[Estribillo]\nla la", LyricsSource::Tags);
        assert_eq!(texts(&lyrics), ["[Estribillo]", "la la"]);
    }
}
