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
    Move,
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
    PlaylistAdd,
    PlaylistDelete,
    PlaylistMove,
    ReadComments,
    Update,
    Partition,
    ListPartitions,
    NewPartition,
    DeletePartition,
    MoveOutput,
    Outputs,
    EnableOutput,
    DisableOutput,
    CommandListBegin,
    CommandListEnd,
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
            Verb::Move => "move",
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
            Verb::PlaylistAdd => "playlistadd",
            Verb::PlaylistDelete => "playlistdelete",
            Verb::PlaylistMove => "playlistmove",
            Verb::ReadComments => "readcomments",
            Verb::Update => "update",
            Verb::Partition => "partition",
            Verb::ListPartitions => "listpartitions",
            Verb::NewPartition => "newpartition",
            Verb::DeletePartition => "delpartition",
            Verb::MoveOutput => "moveoutput",
            Verb::Outputs => "outputs",
            Verb::EnableOutput => "enableoutput",
            Verb::DisableOutput => "disableoutput",
            Verb::CommandListBegin => "command_list_begin",
            Verb::CommandListEnd => "command_list_end",
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
    pub const DISC: &str = "Disc";
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
    pub const PARTITION: &str = "partition";
    pub const OUTPUT_ID: &str = "outputid";
    pub const OUTPUT_NAME: &str = "outputname";
    pub const OUTPUT_PLUGIN: &str = "plugin";
    pub const OUTPUT_ENABLED: &str = "outputenabled";
    /// Comentarios con la letra embebida (Vorbis/FLAC); se comparan sin distinguir
    /// mayúsculas porque `readcomments` devuelve la clave tal como está en el archivo.
    pub const LYRICS_COMMENTS: [&str; 2] = ["LYRICS", "UNSYNCEDLYRICS"];
    /// Pseudo-tag de filtros que coincide con cualquier tag.
    pub const ANY: &str = "any";
}

pub const FLAG_ON: &str = "1";
pub const FLAG_OFF: &str = "0";
pub const SINGLE_ONESHOT: &str = "oneshot";
/// Decimales de los segundos de `seekcur` absoluto: MPD acepta fracciones.
pub const SEEK_DECIMALS: usize = 3;
/// Volumen que informa MPD cuando no hay mixer.
pub const VOLUME_UNAVAILABLE: &str = "-1";
/// Separa los campos de `audio`: `44100:16:2` o `dsd64:2`.
pub const AUDIO_FIELD_SEPARATOR: char = ':';
/// Profundidad de las muestras en punto flotante (`48000:f:2`).
pub const SAMPLE_FORMAT_FLOAT: &str = "f";
/// Prefijo de la tasa DSD (`dsd64`).
pub const DSD_PREFIX: &str = "dsd";
/// Código de `ACK` de lo que no existe (una partición, una salida, una playlist…).
pub const ACK_NO_EXIST: u32 = 50;
/// Plugin con el que una partición lista las salidas que pertenecen a otra.
pub const DUMMY_PLUGIN: &str = "dummy";
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
        "output" => Some(Subsystem::Outputs),
        "partition" => Some(Subsystem::Partitions),
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
