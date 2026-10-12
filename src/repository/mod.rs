//! Repository: acceso a datos. Define los contratos y su implementación sobre MPD.

mod error;
mod fifo;
pub mod mpd;
mod traits;

pub use error::{Ack, Error, Result};
pub use fifo::FifoPcmSource;
pub use traits::{
    ChangesRepository, LibraryRepository, OptionsRepository, PcmSource, PlayerRepository,
    PlaylistRepository, QueueRepository,
};
