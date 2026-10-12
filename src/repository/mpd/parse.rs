//! Conversión de pares `clave: valor` a modelos del dominio.
//! Ante un valor inválido se falla con `Error::Protocol` en lugar de ignorarlo.

use std::str::FromStr;
use std::time::Duration;

use super::protocol::Pair;
use super::vocabulary::{
    AUDIO_FIELD_SEPARATOR, BITRATE_UNKNOWN, DSD_PREFIX, DUMMY_PLUGIN, SAMPLE_FORMAT_FLOAT,
    SINGLE_ONESHOT, VOLUME_UNAVAILABLE, keys, parse_flag, parse_play_state, parse_single_mode,
    parse_subsystem,
};
use crate::domain::{
    AudioFormat, Entry, Output, PartitionName, PlaylistName, SampleFormat, Song, Status, Subsystem,
    Volume,
};
use crate::repository::error::{Error, Result};

pub fn parse_status(pairs: &[Pair]) -> Result<Status> {
    let mut status = Status::default();
    for (key, value) in pairs {
        match key.as_str() {
            keys::VOLUME => status.volume = parse_volume(value)?,
            keys::REPEAT => status.repeat = parse_with(key, value, parse_flag)?,
            keys::RANDOM => status.random = parse_with(key, value, parse_flag)?,
            keys::SINGLE => status.single = parse_with(key, value, parse_single_mode)?,
            keys::CONSUME => status.consume = parse_consume(key, value)?,
            keys::QUEUE_LENGTH => status.queue_length = parse_number(key, value)?,
            keys::STATE => status.state = parse_with(key, value, parse_play_state)?,
            keys::SONG_POSITION => status.song_position = Some(parse_number(key, value)?),
            keys::ELAPSED => status.elapsed = Some(parse_seconds(key, value)?),
            keys::DURATION => status.duration = Some(parse_seconds(key, value)?),
            keys::AUDIO => status.audio = Some(parse_with(key, value, parse_audio_format)?),
            keys::BITRATE => status.bitrate = parse_bitrate(key, value)?,
            keys::UPDATING_DB => status.updating_db = Some(parse_number(key, value)?),
            _ => {}
        }
    }
    Ok(status)
}

/// `frecuencia:bits:canales` (bits `f` = flotante) o `dsdN:canales`.
fn parse_audio_format(value: &str) -> Option<AudioFormat> {
    let fields: Vec<&str> = value.split(AUDIO_FIELD_SEPARATOR).collect();
    match fields.as_slice() {
        [rate, bits, channels] => Some(AudioFormat {
            sample_rate: Some(rate.parse().ok()?),
            format: parse_sample_format(bits)?,
            channels: channels.parse().ok()?,
        }),
        [dsd_rate, channels] => Some(AudioFormat {
            sample_rate: None,
            format: SampleFormat::Dsd {
                rate: dsd_rate.strip_prefix(DSD_PREFIX)?.parse().ok()?,
            },
            channels: channels.parse().ok()?,
        }),
        _ => None,
    }
}

fn parse_sample_format(bits: &str) -> Option<SampleFormat> {
    if bits == SAMPLE_FORMAT_FLOAT {
        return Some(SampleFormat::Float);
    }
    Some(SampleFormat::Integer {
        bits: bits.parse().ok()?,
    })
}

/// `0` significa que MPD no conoce el bitrate.
fn parse_bitrate(key: &str, value: &str) -> Result<Option<u32>> {
    let bitrate: u32 = parse_number(key, value)?;
    Ok((bitrate != BITRATE_UNKNOWN).then_some(bitrate))
}

/// Construye una canción a partir de un registro que empieza con `file`.
pub fn parse_song(record: &[Pair]) -> Result<Song> {
    let mut song = Song::default();
    let mut legacy_duration = None;
    for (key, value) in record {
        match key.as_str() {
            keys::FILE => song.file = value.clone(),
            keys::TITLE => song.title = Some(value.clone()),
            keys::ARTIST => song.artist = Some(value.clone()),
            keys::ALBUM_ARTIST => song.album_artist = Some(value.clone()),
            keys::ALBUM => song.album = Some(value.clone()),
            keys::DATE => song.date = Some(value.clone()),
            keys::TRACK => song.track = Some(value.clone()),
            keys::DURATION => song.duration = Some(parse_seconds(key, value)?),
            keys::LEGACY_TIME => legacy_duration = Some(parse_seconds(key, value)?),
            keys::POSITION => song.position = Some(parse_number(key, value)?),
            _ => {}
        }
    }
    if song.file.is_empty() {
        return Err(Error::Protocol("canción sin campo `file`".into()));
    }
    song.duration = song.duration.or(legacy_duration);
    Ok(song)
}

pub fn parse_songs(pairs: &[Pair]) -> Result<Vec<Song>> {
    split_records(pairs, &[keys::FILE])
        .into_iter()
        .map(parse_song)
        .collect()
}

pub fn parse_entries(pairs: &[Pair]) -> Result<Vec<Entry>> {
    split_records(pairs, &[keys::DIRECTORY, keys::FILE, keys::PLAYLIST])
        .into_iter()
        .map(parse_entry)
        .collect()
}

fn parse_entry(record: &[Pair]) -> Result<Entry> {
    let (key, value) = &record[0];
    Ok(match key.as_str() {
        keys::DIRECTORY => Entry::Directory {
            path: value.clone(),
        },
        keys::PLAYLIST => Entry::Playlist {
            path: value.clone(),
        },
        _ => Entry::File(Box::new(parse_song(record)?)),
    })
}

pub fn parse_playlist_names(pairs: &[Pair]) -> Result<Vec<PlaylistName>> {
    values_of(pairs, keys::PLAYLIST)
        .map(|name| {
            PlaylistName::new(&name)
                .map_err(|error| Error::Protocol(format!("playlist {name:?}: {error}")))
        })
        .collect()
}

pub fn parse_partition_names(pairs: &[Pair]) -> Result<Vec<PartitionName>> {
    values_of(pairs, keys::PARTITION)
        .map(|name| {
            PartitionName::new(&name)
                .map_err(|error| Error::Protocol(format!("partición {name:?}: {error}")))
        })
        .collect()
}

/// Las salidas propias de la partición: las que pertenecen a otra aparecen en su lista
/// con el plugin `dummy` y se descartan.
pub fn parse_outputs(pairs: &[Pair]) -> Result<Vec<Output>> {
    let outputs: Vec<Output> = split_records(pairs, &[keys::OUTPUT_ID])
        .into_iter()
        .map(parse_output)
        .collect::<Result<_>>()?;
    Ok(outputs
        .into_iter()
        .filter(|output| output.plugin != DUMMY_PLUGIN)
        .collect())
}

fn parse_output(record: &[Pair]) -> Result<Output> {
    let (key, value) = &record[0];
    let mut output = Output {
        id: parse_number(key, value)?,
        name: String::new(),
        plugin: String::new(),
        enabled: false,
    };
    for (key, value) in &record[1..] {
        match key.as_str() {
            keys::OUTPUT_NAME => output.name = value.clone(),
            keys::OUTPUT_PLUGIN => output.plugin = value.clone(),
            keys::OUTPUT_ENABLED => output.enabled = parse_with(key, value, parse_flag)?,
            _ => {}
        }
    }
    if output.name.is_empty() {
        return Err(Error::Protocol(format!("salida {} sin nombre", output.id)));
    }
    Ok(output)
}

pub fn parse_subsystems(pairs: &[Pair]) -> Vec<Subsystem> {
    values_of(pairs, keys::CHANGED)
        .filter_map(|name| parse_subsystem(&name))
        .collect()
}

/// Valores de todas las apariciones de `key`, sin distinguir mayúsculas.
pub fn values_of<'a>(pairs: &'a [Pair], key: &'a str) -> impl Iterator<Item = String> + 'a {
    pairs
        .iter()
        .filter(move |(pair_key, _)| pair_key.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.clone())
}

/// Parte una respuesta en registros: cada clave de `starters` abre un registro nuevo.
/// Los pares previos al primer inicio se descartan.
fn split_records<'a>(pairs: &'a [Pair], starters: &[&str]) -> Vec<&'a [Pair]> {
    let starts: Vec<usize> = pairs
        .iter()
        .enumerate()
        .filter(|(_, (key, _))| starters.contains(&key.as_str()))
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(record_number, &start)| {
            let end = starts
                .get(record_number + 1)
                .copied()
                .unwrap_or(pairs.len());
            &pairs[start..end]
        })
        .collect()
}

fn parse_volume(value: &str) -> Result<Option<Volume>> {
    if value == VOLUME_UNAVAILABLE {
        return Ok(None);
    }
    let raw: u8 = parse_number(keys::VOLUME, value)?;
    Volume::new(raw)
        .map(Some)
        .map_err(|error| Error::Protocol(error.to_string()))
}

/// `consume` admite `oneshot` desde MPD 0.24; para el cliente cuenta como activo.
fn parse_consume(key: &str, value: &str) -> Result<bool> {
    if value == SINGLE_ONESHOT {
        return Ok(true);
    }
    parse_with(key, value, parse_flag)
}

fn parse_with<T>(key: &str, value: &str, parser: impl Fn(&str) -> Option<T>) -> Result<T> {
    parser(value).ok_or_else(|| invalid_value(key, value))
}

fn parse_number<T: FromStr>(key: &str, value: &str) -> Result<T> {
    value.parse().map_err(|_| invalid_value(key, value))
}

fn parse_seconds(key: &str, value: &str) -> Result<Duration> {
    let seconds: f64 = parse_number(key, value)?;
    Duration::try_from_secs_f64(seconds).map_err(|_| invalid_value(key, value))
}

fn invalid_value(key: &str, value: &str) -> Error {
    Error::Protocol(format!("valor inválido en {key}: {value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PlayState, SingleMode};

    fn pairs(raw: &str) -> Vec<Pair> {
        raw.lines()
            .map(|line| {
                let (key, value) = line.split_once(": ").unwrap();
                (key.to_owned(), value.to_owned())
            })
            .collect()
    }

    #[test]
    fn status_parses_playing_state() {
        let status = parse_status(&pairs(
            "volume: 100\nrepeat: 0\nrandom: 1\nsingle: oneshot\nconsume: 0\n\
             playlistlength: 1229\nstate: play\nsong: 1186\nelapsed: 51.250\nduration: 208.000",
        ))
        .unwrap();
        assert_eq!(status.volume, Some(Volume::new(100).unwrap()));
        assert!(status.random && !status.repeat && !status.consume);
        assert_eq!(status.single, SingleMode::Oneshot);
        assert_eq!(status.state, PlayState::Play);
        assert_eq!(status.song_position, Some(1186));
        assert_eq!(status.elapsed, Some(Duration::from_millis(51_250)));
        assert_eq!(status.queue_length, 1229);
    }

    #[test]
    fn status_without_mixer_has_no_volume() {
        let status = parse_status(&pairs("volume: -1\nstate: stop")).unwrap();
        assert_eq!(status.volume, None);
        assert_eq!(status.song_position, None);
    }

    #[test]
    fn status_fails_fast_on_invalid_values() {
        assert!(parse_status(&pairs("state: bailando")).is_err());
        assert!(parse_status(&pairs("volume: 150")).is_err());
        assert!(parse_status(&pairs("repeat: tal vez")).is_err());
        assert!(parse_status(&pairs("elapsed: -3")).is_err());
    }

    fn audio_of(value: &str) -> Result<Option<AudioFormat>> {
        parse_status(&pairs(&format!("audio: {value}"))).map(|status| status.audio)
    }

    #[test]
    fn status_parses_audio_formats() {
        let pcm = |sample_rate, format| AudioFormat {
            sample_rate: Some(sample_rate),
            format,
            channels: 2,
        };
        let dsd = |rate| AudioFormat {
            sample_rate: None,
            format: SampleFormat::Dsd { rate },
            channels: 2,
        };
        let cases = [
            (
                "44100:16:2",
                pcm(44_100, SampleFormat::Integer { bits: 16 }),
            ),
            (
                "96000:24:2",
                pcm(96_000, SampleFormat::Integer { bits: 24 }),
            ),
            ("48000:f:2", pcm(48_000, SampleFormat::Float)),
            ("dsd64:2", dsd(64)),
            ("dsd128:2", dsd(128)),
        ];
        for (value, expected) in cases {
            assert_eq!(audio_of(value).unwrap(), Some(expected), "{value}");
        }
    }

    #[test]
    fn invalid_audio_formats_fail_fast() {
        for value in [
            "44100:16",
            "abc:16:2",
            "44100:x:2",
            "dsd:2",
            "pcm64:2",
            "1:2:3:4",
        ] {
            assert!(audio_of(value).is_err(), "{value}");
        }
    }

    #[test]
    fn zero_bitrate_is_unknown() {
        let bitrate = |raw: &str| parse_status(&pairs(raw)).unwrap().bitrate;
        assert_eq!(bitrate("bitrate: 1411"), Some(1411));
        assert_eq!(bitrate("bitrate: 0"), None);
        assert_eq!(bitrate("state: stop"), None);
        assert!(parse_status(&pairs("bitrate: rápido")).is_err());
    }

    #[test]
    fn song_falls_back_to_legacy_time_and_file_name() {
        let song = parse_song(&pairs("file: Rock/01 - Tema.mp3\nTime: 200\nPos: 3")).unwrap();
        assert_eq!(song.duration, Some(Duration::from_secs(200)));
        assert_eq!(song.display_title(), "01 - Tema.mp3");
        assert_eq!(song.position, Some(3));
    }

    #[test]
    fn song_keeps_album_artist_and_free_date() {
        let song = parse_song(&pairs(
            "file: a.mp3\nAlbumArtist: Varios\nDate: 1973-03-01\nAlbum: Éxitos",
        ))
        .unwrap();
        assert_eq!(song.album_artist.as_deref(), Some("Varios"));
        assert_eq!(song.date.as_deref(), Some("1973-03-01"));
        let odd = parse_song(&pairs("file: b.mp3\nDate: circa 70s")).unwrap();
        assert_eq!(odd.date.as_deref(), Some("circa 70s"));
    }

    #[test]
    fn songs_split_on_file_key() {
        let songs = parse_songs(&pairs(
            "file: a.mp3\nTitle: A\nduration: 1.5\nfile: b.mp3\nTitle: B",
        ))
        .unwrap();
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[1].title.as_deref(), Some("B"));
    }

    #[test]
    fn entries_distinguish_kinds() {
        let entries = parse_entries(&pairs(
            "directory: Rock\nLast-Modified: 2020-01-01T00:00:00Z\nfile: x.mp3\nTitle: X\nplaylist: lista.m3u",
        ))
        .unwrap();
        assert_eq!(entries.len(), 3);
        assert!(matches!(&entries[0], Entry::Directory { path } if path == "Rock"));
        assert!(matches!(&entries[1], Entry::File(song) if song.file == "x.mp3"));
        assert!(matches!(&entries[2], Entry::Playlist { path } if path == "lista.m3u"));
    }

    #[test]
    fn playlist_names_are_validated() {
        let names = parse_playlist_names(&pairs(
            "playlist: uno\nLast-Modified: 2024-05-01T10:00:00Z\nplaylist: dos",
        ))
        .unwrap();
        assert_eq!(
            names,
            [
                PlaylistName::new("uno").unwrap(),
                PlaylistName::new("dos").unwrap()
            ]
        );
    }

    #[test]
    fn unknown_subsystems_are_ignored() {
        let changed = parse_subsystems(&pairs("changed: player\nchanged: neighbor"));
        assert_eq!(changed, [Subsystem::Player]);
    }

    #[test]
    fn output_and_partition_subsystems_are_parsed() {
        assert_eq!(
            parse_subsystems(&pairs("changed: output\nchanged: partition")),
            [Subsystem::Outputs, Subsystem::Partitions]
        );
    }

    #[test]
    fn update_subsystem_and_job_are_parsed() {
        assert_eq!(
            parse_subsystems(&pairs("changed: update")),
            [Subsystem::Update]
        );
        let updating = |raw: &str| parse_status(&pairs(raw)).map(|status| status.updating_db);
        assert_eq!(updating("updating_db: 3").unwrap(), Some(3));
        assert_eq!(updating("state: play").unwrap(), None);
        assert!(updating("updating_db: sí").is_err());
    }

    #[test]
    fn outputs_skip_the_ones_owned_by_another_partition() {
        let outputs = parse_outputs(&pairs(
            "outputid: 0\noutputname: pulse audio\nplugin: pulse\noutputenabled: 1\n\
             outputid: 1\noutputname: alsa hw:1,3\nplugin: alsa\noutputenabled: 0\n\
             attribute: dop=0\n\
             outputid: 2\noutputname: FIFO pipe\nplugin: dummy\noutputenabled: 1",
        ))
        .unwrap();
        let names: Vec<_> = outputs.iter().map(|output| output.name.as_str()).collect();
        assert_eq!(names, ["pulse audio", "alsa hw:1,3"]);
        assert_eq!(outputs[1].id, 1);
        assert_eq!(outputs[1].plugin, "alsa");
        assert!(outputs[0].enabled && !outputs[1].enabled);
    }

    #[test]
    fn outputs_fail_fast_on_invalid_values() {
        assert!(parse_outputs(&pairs("outputid: x\noutputname: a")).is_err());
        assert!(parse_outputs(&pairs("outputid: 0\noutputname: a\noutputenabled: 2")).is_err());
        assert!(parse_outputs(&pairs("outputid: 0\nplugin: alsa")).is_err());
    }

    #[test]
    fn partition_names_are_validated() {
        let names = parse_partition_names(&pairs("partition: default\npartition: cocina")).unwrap();
        assert_eq!(names[1].as_str(), "cocina");
        assert!(parse_partition_names(&pairs("partition: la cocina")).is_err());
    }
}
