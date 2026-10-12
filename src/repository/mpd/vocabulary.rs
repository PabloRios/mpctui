//! Vocabulario del protocolo de MPD: comandos, claves y valores de respuesta.

use crate::domain::{PlayState, SingleMode, Subsystem, Tag};

/// Comandos que envía el cliente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Password,
    Ping,
    Status,
    CurrentSong,
    Idle,
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    SetVolume,
    SeekCurrent,
    Random,
    Repeat,
    Single,
    Consume,
    QueueInfo,
    Add,
    FindAdd,
    Delete,
    Clear,
    ListInfo,
    List,
    Find,
    Search,
    ListPlaylists,
    ListPlaylistInfo,
    Load,
    Save,
    Remove,
}

impl Verb {
    pub fn as_str(self) -> &'static str {
        match self {
            Verb::Password => "password",
            Verb::Ping => "ping",
            Verb::Status => "status",
            Verb::CurrentSong => "currentsong",
            Verb::Idle => "idle",
            Verb::Play => "play",
            Verb::Pause => "pause",
            Verb::Stop => "stop",
            Verb::Next => "next",
            Verb::Previous => "previous",
            Verb::SetVolume => "setvol",
            Verb::SeekCurrent => "seekcur",
            Verb::Random => "random",
            Verb::Repeat => "repeat",
            Verb::Single => "single",
            Verb::Consume => "consume",
            Verb::QueueInfo => "playlistinfo",
            Verb::Add => "add",
            Verb::FindAdd => "findadd",
            Verb::Delete => "delete",
            Verb::Clear => "clear",
            Verb::ListInfo => "lsinfo",
            Verb::List => "list",
            Verb::Find => "find",
            Verb::Search => "search",
            Verb::ListPlaylists => "listplaylists",
            Verb::ListPlaylistInfo => "listplaylistinfo",
            Verb::Load => "load",
            Verb::Save => "save",
            Verb::Remove => "rm",
        }
    }
}

/// Claves de los pares `clave: valor` de las respuestas.
pub mod keys {
    pub const FILE: &str = "file";
    pub const DIRECTORY: &str = "directory";
    pub const PLAYLIST: &str = "playlist";
    pub const TITLE: &str = "Title";
    pub const ARTIST: &str = "Artist";
    pub const ALBUM_ARTIST: &str = "AlbumArtist";
    pub const ALBUM: &str = "Album";
    pub const DATE: &str = "Date";
    pub const TRACK: &str = "Track";
    pub const DURATION: &str = "duration";
    pub const LEGACY_TIME: &str = "Time";
    pub const POSITION: &str = "Pos";
    pub const VOLUME: &str = "volume";
    pub const REPEAT: &str = "repeat";
    pub const RANDOM: &str = "random";
    pub const SINGLE: &str = "single";
    pub const CONSUME: &str = "consume";
    pub const QUEUE_LENGTH: &str = "playlistlength";
    pub const STATE: &str = "state";
    pub const SONG_POSITION: &str = "song";
    pub const ELAPSED: &str = "elapsed";
    pub const AUDIO: &str = "audio";
    pub const BITRATE: &str = "bitrate";
    pub const UPDATING_DB: &str = "updating_db";
    pub const CHANGED: &str = "changed";
    /// Pseudo-tag de filtros que coincide con cualquier tag.
    pub const ANY: &str = "any";
}

pub const FLAG_ON: &str = "1";
pub const FLAG_OFF: &str = "0";
pub const SINGLE_ONESHOT: &str = "oneshot";
/// Volumen que informa MPD cuando no hay mixer.
pub const VOLUME_UNAVAILABLE: &str = "-1";
/// Separa los campos de `audio`: `44100:16:2` o `dsd64:2`.
pub const AUDIO_FIELD_SEPARATOR: char = ':';
/// Profundidad de las muestras en punto flotante (`48000:f:2`).
pub const SAMPLE_FORMAT_FLOAT: &str = "f";
/// Prefijo de la tasa DSD (`dsd64`).
pub const DSD_PREFIX: &str = "dsd";
/// Bitrate que MPD informa cuando no lo conoce.
pub const BITRATE_UNKNOWN: u32 = 0;

pub fn flag(enabled: bool) -> &'static str {
    if enabled { FLAG_ON } else { FLAG_OFF }
}

pub fn single_mode_argument(mode: SingleMode) -> &'static str {
    match mode {
        SingleMode::Off => FLAG_OFF,
        SingleMode::On => FLAG_ON,
        SingleMode::Oneshot => SINGLE_ONESHOT,
    }
}

pub fn parse_single_mode(value: &str) -> Option<SingleMode> {
    match value {
        FLAG_OFF => Some(SingleMode::Off),
        FLAG_ON => Some(SingleMode::On),
        SINGLE_ONESHOT => Some(SingleMode::Oneshot),
        _ => None,
    }
}

pub fn parse_flag(value: &str) -> Option<bool> {
    match value {
        FLAG_OFF => Some(false),
        FLAG_ON => Some(true),
        _ => None,
    }
}

pub fn parse_play_state(value: &str) -> Option<PlayState> {
    match value {
        "play" => Some(PlayState::Play),
        "pause" => Some(PlayState::Pause),
        "stop" => Some(PlayState::Stop),
        _ => None,
    }
}

/// Subsistemas que no interesan al cliente devuelven `None`.
pub fn parse_subsystem(name: &str) -> Option<Subsystem> {
    match name {
        "database" => Some(Subsystem::Database),
        "stored_playlist" => Some(Subsystem::StoredPlaylist),
        "playlist" => Some(Subsystem::Queue),
        "player" => Some(Subsystem::Player),
        "mixer" => Some(Subsystem::Mixer),
        "options" => Some(Subsystem::Options),
        "update" => Some(Subsystem::Update),
        _ => None,
    }
}

pub fn tag_name(tag: Tag) -> &'static str {
    match tag {
        Tag::Artist => keys::ARTIST,
        Tag::AlbumArtist => keys::ALBUM_ARTIST,
        Tag::Album => keys::ALBUM,
        Tag::Title => keys::TITLE,
        Tag::Any => keys::ANY,
    }
}
