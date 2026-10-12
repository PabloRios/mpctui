//! Repository: acceso a datos. Define los contratos y su implementación sobre MPD.

mod error;
mod fifo;
mod lrclib;
mod lyrics_files;
pub mod mpd;
mod traits;

pub use error::{Ack, Error, HttpFailure, IoFailure, Result};
pub use fifo::FifoPcmSource;
pub use lrclib::{LRCLIB_URL, LrclibProvider};
pub use lyrics_files::LocalLyricsFiles;
pub use traits::{
    ChangesRepository, EmbeddedLyricsRepository, LibraryRepository, LyricsFiles, LyricsProvider,
    OptionsRepository, PcmSource, PlayerRepository, PlaylistRepository, QueueRepository,
};
