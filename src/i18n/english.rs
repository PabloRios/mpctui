//! Textos en inglés, el idioma por defecto.

use std::io;

use super::{Catalog, Message, Phrase, counted, seconds_with_decimal};
use crate::config::{
    ConfigError, GLYPHS_VARIABLE, PORT_VARIABLE, glyph_choices, language_choices,
    lyrics_source_choices, visualizer_style_choices,
};
use crate::domain::{
    InputError, MAX_VOLUME, PATH_SEPARATOR, PCM_BITS, PCM_MAX_CHANNELS, PCM_MAX_SAMPLE_RATE,
    PCM_MIN_SAMPLE_RATE,
};
use crate::repository::{Error, HttpFailure};

const DECIMAL_SEPARATOR: char = '.';

#[derive(Debug, Clone, Copy, Default)]
pub struct English;

impl Catalog for English {
    fn phrase(&self, phrase: Phrase) -> &'static str {
        match phrase {
            Phrase::Quit => "Quit",
            Phrase::ToggleHelp => "Show or hide this help",
            Phrase::GoToView => "Go to view",
            Phrase::CycleVisualizer => "Visualizer / next style",
            Phrase::NextView => "Next view",
            Phrase::PreviousView => "Previous view",
            Phrase::MoveDown => "Move down",
            Phrase::MoveUp => "Move up",
            Phrase::PageDown => "Page down",
            Phrase::PageUp => "Page up",
            Phrase::GoToFirst => "Go to the top",
            Phrase::GoToLast => "Go to the bottom",
            Phrase::Activate => "Play / open / load",
            Phrase::Forward => "Open / next column",
            Phrase::Back => "Back / previous column",
            Phrase::AddSelection => "Add (playlist: load)",
            Phrase::AddAll => "Add all",
            Phrase::DeleteSelection => "Delete (playlist: asks)",
            Phrase::TogglePause => "Pause / resume",
            Phrase::Stop => "Stop",
            Phrase::NextSong => "Next song",
            Phrase::PreviousSong => "Previous song",
            Phrase::VolumeUp => "Volume up",
            Phrase::VolumeDown => "Volume down",
            Phrase::SeekForward => "Seek forward",
            Phrase::SeekBackward => "Seek backward",
            Phrase::ClearQueue => "Clear the queue (asks)",
            Phrase::SaveQueue => "Save the queue as a playlist",
            Phrase::Search => "Search",
            Phrase::Random => "Random",
            Phrase::Repeat => "Repeat",
            Phrase::CycleSingle => "Single: off, on, once",
            Phrase::Consume => "Consume",
            Phrase::ShowPlaying => "Go to the playing song",
            Phrase::MoveSong => "Move the song down / up",
            Phrase::AddToPlaylist => "Add to a playlist",
            Phrase::RetryLyrics => "Retry the lyrics download",
            Phrase::UpdateDatabase => "Update the library",
            Phrase::NewPartition => "New partition",
            Phrase::MoveOutput => "Move output to partition",
            Phrase::HelpNavigation => "Navigation",
            Phrase::HelpLists => "Queue and lists",
            Phrase::HelpPlayback => "Playback",
            Phrase::HelpModes => "Modes",
            Phrase::HelpOther => "Other",
            Phrase::HelpTitle => "Help",
            Phrase::HelpClose => "any key closes",
            Phrase::KeySpace => "Space",
            Phrase::KeyPageUp => "PgUp",
            Phrase::KeyPageDown => "PgDn",
            Phrase::KeyHome => "Home",
            Phrase::KeyEnd => "End",
            Phrase::KeyBackspace => "Backspace",
            Phrase::KeyDelete => "Del",
            Phrase::KeyUp => "Up",
            Phrase::KeyDown => "Down",
            Phrase::KeyLeft => "Left",
            Phrase::KeyRight => "Right",
            Phrase::HintPlay => "play",
            Phrase::HintDelete => "delete",
            Phrase::HintShowPlaying => "go to current",
            Phrase::HintSearch => "search",
            Phrase::HintHelp => "help",
            Phrase::HintOpen => "open",
            Phrase::HintBack => "back",
            Phrase::HintAdd => "add",
            Phrase::HintAddAlbum => "add album",
            Phrase::HintOpenOrAdd => "open or add",
            Phrase::HintParent => "up",
            Phrase::HintAddAll => "add all",
            Phrase::HintLoad => "load",
            Phrase::HintSaveQueue => "save queue",
            Phrase::HintMoveDown => "move down",
            Phrase::HintMoveUp => "move up",
            Phrase::HintEdit => "edit",
            Phrase::HintRemove => "remove",
            Phrase::HintToPlaylist => "to playlist",
            Phrase::HintOutputs => "outputs",
            Phrase::HintNewPartition => "new partition",
            Phrase::HintToggleOutput => "enable/disable",
            Phrase::HintMoveOutput => "move",
            Phrase::HintPause => "pause",
            Phrase::HintNextStyle => "next style",
            Phrase::HintRetry => "retry",
            Phrase::HintPlayFromLine => "play from here",
            Phrase::HintFollowSong => "follow the song",
            Phrase::SearchKeys => "Enter searches   Tab changes the field   Esc cancels",
            Phrase::SaveQueueTitle => "Save queue as playlist",
            Phrase::ConfirmTitle => "Confirm",
            Phrase::NameLabel => "Name",
            Phrase::SaveKeys => "Enter saves   Esc cancels",
            Phrase::NewPlaylistTitle => "New playlist",
            Phrase::PickerTitle => "Add to playlist",
            Phrase::PickerKeys => "Enter chooses   Esc cancels",
            Phrase::NewPlaylistChoice => "New playlist",
            Phrase::NewPartitionTitle => "New partition",
            Phrase::PartitionPickerTitle => "Move to partition",
            Phrase::Yes => "yes",
            Phrase::No => "no",
            Phrase::ConfirmKey => "y",
            Phrase::ViewQueue => "Queue",
            Phrase::ViewLibrary => "Library",
            Phrase::ViewLibraryShort => "Lib.",
            Phrase::ViewBrowse => "Directories",
            Phrase::ViewBrowseShort => "Dir.",
            Phrase::ViewSearch => "Search",
            Phrase::ViewSearchShort => "Srch.",
            Phrase::ViewPlaylists => "Playlists",
            Phrase::ViewPlaylistsShort => "Playl.",
            Phrase::ViewVisualizer => "Visual",
            Phrase::ViewLyrics => "Lyrics",
            Phrase::ViewOutputs => "Outputs",
            Phrase::ViewOutputsShort => "Out.",
            Phrase::Disconnected => "disconnected",
            Phrase::Connecting => "connecting",
            Phrase::Playing => "playing",
            Phrase::Paused => "paused",
            Phrase::Stopped => "stopped",
            Phrase::NothingPlaying => "nothing playing",
            Phrase::Live => "live",
            Phrase::VolumeUnavailable => "n/a",
            Phrase::VolumeLabel => "vol",
            Phrase::PositionLabel => "pos",
            Phrase::SongLabel => "song",
            Phrase::AlbumLabel => "album",
            Phrase::QueueLabel => "queue",
            Phrase::ModeRandom => "random",
            Phrase::ModeRepeat => "repeat",
            Phrase::ModeSingle => "single",
            Phrase::ModeConsume => "consume",
            Phrase::Unnamed => "(unnamed)",
            Phrase::NoAlbum => "[no album]",
            Phrase::TagAny => "All",
            Phrase::TagArtist => "Artist",
            Phrase::TagAlbumArtist => "Album artist",
            Phrase::TagAlbum => "Album",
            Phrase::TagTitle => "Title",
            Phrase::ColumnPosition => "pos",
            Phrase::ColumnDuration => "time",
            Phrase::ColumnSong => "Song",
            Phrase::PartitionsTitle => "Partitions",
            Phrase::Mono => "mono",
            Phrase::Stereo => "stereo",
            Phrase::FloatSamples => "float",
            Phrase::StyleBars => "Bars",
            Phrase::StyleMirror => "Stereo mirror",
            Phrase::StyleWave => "Wave",
            Phrase::StyleWaterfall => "Waterfall",
            Phrase::StyleVectorscope => "Vectorscope",
            Phrase::StyleMeters => "Meters",
            Phrase::VisualizerTitle => "Visualizer",
            Phrase::NoSignal => "no signal",
            Phrase::FifoAddOutput => "Add a fifo output to mpd.conf and restart MPD:",
            Phrase::FifoOutputName => "visualizer",
            Phrase::FifoOtherPath => {
                "If you use another path or format, set visualizer_fifo and visualizer_format."
            }
            Phrase::MetersDetail => "RMS and peak per channel",
            Phrase::Peak => "peak",
            Phrase::Now => "now",
            Phrase::Correlation => "correlation",
            Phrase::NoSong => "No song playing",
            Phrase::SearchingLyrics => "searching lyrics",
            Phrase::Instrumental => "Instrumental",
            Phrase::UntitledHint => "The song has no artist or title in its tags.",
            Phrase::LastError => "Last error",
            Phrase::Synced => "synced",
            Phrase::PlainText => "plain text",
            Phrase::SourceCache => "cache",
            Phrase::SourceSongFolder => "song folder",
            Phrase::SourceTags => "tags",
            Phrase::FieldLabel => "field",
            Phrase::FieldHint => "Tab changes",
            Phrase::ConnectionRecovered => "Connection restored",
            Phrase::LyricsDisabled => "Lyrics are disabled (lyrics.enabled = false)",
            Phrase::LibraryUpdateRequested => "Library update requested",
            Phrase::QueueCleared => "Queue cleared",
            Phrase::NoVolumeControl => "The server has no volume control",
            Phrase::DefaultPartitionUndeletable => "The default partition cannot be deleted",
            Phrase::TerminalTooSmall => "The terminal is too small.",
        }
    }

    fn message(&self, message: &Message<'_>) -> String {
        match *message {
            Message::SongCount(count) => songs(count),
            Message::EntryCount(count) => counted(count, "entry", "entries"),
            Message::OutputCount(count) => counted(count, "output", "outputs"),
            Message::ChannelCount(count) => format!("{count} channels"),
            Message::ResultsInAlbums { results, albums } => format!(
                "{} in {}",
                counted(results, "result", "results"),
                counted(albums, "album", "albums")
            ),
            Message::UpdatingLibrary { elapsed } => format!("updating library {elapsed}"),
            Message::RetryIn { countdown } => format!("retrying in {countdown}"),
            Message::Attempt(attempt) => format!("(attempt {attempt})"),
            Message::QueueSummary(count) => format!("queue: {}", songs(count)),
            Message::ConfirmClearQueue(count) => {
                format!("Clear the queue? {} will be removed.", songs(count))
            }
            Message::ConfirmRemovePlaylist(name) => format!("Delete the playlist \"{name}\"?"),
            Message::ConfirmRemovePartition { name, outputs } => match outputs {
                0 => format!("Delete the partition \"{name}\"?"),
                1 => format!("Delete the partition \"{name}\"? Its output returns to default."),
                many => format!(
                    "Delete the partition \"{name}\"? Its {many} outputs return to default."
                ),
            },
            Message::TerminalSize {
                minimum_width,
                minimum_height,
                width,
                height,
            } => {
                format!("Minimum: {minimum_width} x {minimum_height}. Current: {width} x {height}.")
            }
            Message::EmptyQueue { key } => {
                format!("The queue is empty. {key} opens the library to add songs.")
            }
            Message::NoPlaylists { key } => {
                format!("No saved playlists. {key} saves the queue as a playlist.")
            }
            Message::NoOutputs { key } => {
                format!("No outputs. Select one in another partition and move it here with {key}.")
            }
            Message::SearchHint { key } => format!("{key} searches the library"),
            Message::NoResults { query } => format!("No results for \"{query}\""),
            Message::LyricsNotFound { title } => format!("No lyrics found for \"{title}\""),
            Message::CannotRead { source, error } => {
                format!("Cannot read {source} ({}).", unreadable(error))
            }
            Message::BarsDetail { bands, low, high } => format!("{bands} bands  {low} - {high}"),
            Message::MirrorDetail { bands } => {
                format!("{bands} bands per channel  L up, R down")
            }
            Message::WaveDetail { span, synced } => {
                let sync = if synced { "synced" } else { "free-running" };
                format!("{} ms  {sync}", span.as_millis())
            }
            Message::WaterfallDetail { bands, seconds } => {
                format!("{bands} bands  last {seconds} s")
            }
            Message::VectorscopeDetail { points } => format!("M/S  {points} samples"),
            Message::MetersLegend {
                window,
                peak_glyph,
                hold,
            } => format!(
                "bar = RMS ({} ms)   {peak_glyph} = held peak {}",
                window.as_millis(),
                seconds_with_decimal(hold, DECIMAL_SEPARATOR)
            ),
            Message::VisualizerChanged { style, key } => {
                format!("Visualizer: {style} ({key}: next style)")
            }
            Message::ConnectionLost(error) => {
                format!("Lost connection to MPD: {}", failure(error))
            }
            Message::AddedToQueue(what) => format!("Added to the queue: {what}"),
            Message::SongsAdded(count) => format!("Added to the queue: {}", songs(count)),
            Message::ResultsFound(count) => counted(count, "result", "results"),
            Message::QueueSaved(name) => format!("Queue saved as \"{name}\""),
            Message::PlaylistDeleted(name) => format!("Playlist \"{name}\" deleted"),
            Message::PlaylistLoaded(name) => format!("Playlist \"{name}\" added to the queue"),
            Message::AddedToPlaylist { what, playlist } => {
                format!("Added to \"{playlist}\": {what}")
            }
            Message::RemovedFromPlaylist { song, playlist } => {
                format!("Removed from \"{playlist}\": {song}")
            }
            Message::LibraryUpdated { elapsed } => format!("Library updated ({elapsed})"),
            Message::NoOtherPartition { key } => {
                format!("There is no other partition: {key} creates one")
            }
            Message::PartitionCreated(name) => format!("Partition \"{name}\" created"),
            Message::PartitionDeleted(name) => format!("Partition \"{name}\" deleted"),
            Message::OutputMoved { output, partition } => {
                format!("\"{output}\" moved to \"{partition}\"")
            }
            Message::OutputToggled { output, enabled } => {
                let state = if enabled { "enabled" } else { "disabled" };
                format!("Output \"{output}\" {state}")
            }
            Message::Failure(error) => failure(error),
            Message::Input(error) => input(error),
            Message::Config(error) => config(error),
            Message::ConnectFailed { address, error } => {
                format!("could not connect to MPD at {address}: {}", failure(error))
            }
        }
    }
}

fn songs(count: usize) -> String {
    counted(count, "song", "songs")
}

/// Por qué no se puede leer la salida fifo.
fn unreadable(error: &Error) -> String {
    match error {
        Error::Io(io) if io.kind() == io::ErrorKind::NotFound => "does not exist".to_owned(),
        Error::Io(io) if io.kind() == io::ErrorKind::PermissionDenied => {
            "no read permission".to_owned()
        }
        Error::Io(io) => io.to_string(),
        other => failure(other),
    }
}

fn failure(error: &Error) -> String {
    match error {
        Error::Io(io) => format!("I/O error: {io}"),
        Error::Ack(ack) => format!("MPD: {ack}"),
        Error::Protocol(detail) => format!("invalid response from MPD: {detail}"),
        Error::Closed => "the connection to MPD was closed".to_owned(),
        Error::Http { service, failure } => format!("{service}: {}", http(failure)),
    }
}

fn http(failure: &HttpFailure) -> String {
    match failure {
        HttpFailure::Timeout => "timed out".to_owned(),
        HttpFailure::Connect => "could not connect".to_owned(),
        HttpFailure::InvalidResponse => "invalid response".to_owned(),
        HttpFailure::Status(code) => format!("answered {code}"),
        HttpFailure::Other(detail) => detail.clone(),
    }
}

fn input(error: &InputError) -> String {
    match error {
        InputError::VolumeOutOfRange(value) => {
            format!("the volume must be between 0 and {MAX_VOLUME}; got {value}")
        }
        InputError::EmptyText => "the text cannot be empty".to_owned(),
        InputError::ControlCharacter => {
            "the text cannot contain line breaks or control characters".to_owned()
        }
        InputError::PathSeparatorInName => {
            format!("the playlist name cannot contain '{PATH_SEPARATOR}'")
        }
        InputError::InvalidPartitionName => {
            "the partition name only accepts ASCII letters and digits, '-' and '_'".to_owned()
        }
        InputError::MalformedPcmFormat(value) => format!(
            "the PCM format must be rate:bits:channels (like \"44100:16:2\"), not {value:?}"
        ),
        InputError::SampleRateOutOfRange(rate) => format!(
            "the sample rate must be between {PCM_MIN_SAMPLE_RATE} and {PCM_MAX_SAMPLE_RATE} Hz; got {rate}"
        ),
        InputError::UnsupportedBits(bits) => {
            format!("only {PCM_BITS}-bit samples are supported; got {bits}")
        }
        InputError::UnsupportedChannels(channels) => {
            format!("only 1 or {PCM_MAX_CHANNELS} channels are supported; got {channels}")
        }
    }
}

fn config(error: &ConfigError) -> String {
    match error {
        ConfigError::Read { path, source } => {
            format!("could not read {}: {source}", path.display())
        }
        ConfigError::Parse { path, source } => {
            format!("invalid configuration in {}: {source}", path.display())
        }
        ConfigError::InvalidPort(value) => {
            format!("{PORT_VARIABLE} is not a valid port: {value:?}")
        }
        ConfigError::EmptyHost => "the MPD host cannot be empty".to_owned(),
        ConfigError::InvalidPassword(error) => format!("invalid MPD password: {}", input(error)),
        ConfigError::InvalidGlyphs(setting) => format!(
            "{GLYPHS_VARIABLE} must be one of {}, not {:?}",
            glyph_choices(),
            setting.0
        ),
        ConfigError::InvalidLanguage(value) => format!(
            "language must be one of {}, not {value:?}",
            language_choices()
        ),
        ConfigError::EmptyVisualizerFifo => "visualizer_fifo cannot be empty".to_owned(),
        ConfigError::InvalidVisualizerFormat(error) => {
            format!("invalid visualizer_format: {}", input(error))
        }
        ConfigError::InvalidVisualizerStyle(value) => format!(
            "visualizer_style must be one of {}, not {value:?}",
            visualizer_style_choices()
        ),
        ConfigError::StereoStyleWithMonoFifo(value) => {
            format!("visualizer_style = {value:?} needs two channels and visualizer_format has one")
        }
        ConfigError::UnknownLyricsSource(value) => format!(
            "lyrics.sources accepts {}, not {value:?}",
            lyrics_source_choices()
        ),
        ConfigError::RepeatedLyricsSource(value) => format!("lyrics.sources repeats {value:?}"),
        ConfigError::EmptyLyricsSources => {
            "lyrics.sources cannot be empty while lyrics are enabled".to_owned()
        }
        ConfigError::SongFolderWithoutMusicDirectory => {
            "lyrics.sources includes \"song_folder\" but lyrics.music_directory is missing"
                .to_owned()
        }
        ConfigError::EmptyLyricsPath(field) => format!("lyrics.{field} cannot be empty"),
        ConfigError::UnknownHomeDirectory(field) => {
            format!("could not find the home directory for lyrics.{field}")
        }
    }
}
