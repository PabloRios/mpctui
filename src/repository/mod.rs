//! Repository: acceso a datos. Define los contratos y su implementación sobre MPD.

mod error;
mod fifo;
mod lyrics_files;
pub mod mpd;
mod traits;

pub use error::{Ack, Error, Result};
pub use fifo::FifoPcmSource;
pub use lyrics_files::LocalLyricsFiles;
pub use traits::{
    ChangesRepository, EmbeddedLyricsRepository, LibraryRepository, LyricsFiles, OptionsRepository,
    PcmSource, PlayerRepository, PlaylistRepository, QueueRepository,
};
