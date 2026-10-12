//! Dominio: modelos compartidos y entradas validadas. No depende de ninguna otra capa.

mod audio;
mod inputs;
mod models;
mod spectrum;
mod timeline;

pub use audio::{AudioFormat, SampleFormat};
pub(crate) use inputs::PATH_SEPARATOR;
pub use inputs::{InputError, MAX_VOLUME, Password, PcmFormat, PlaylistName, SearchText, Volume};
pub use models::{Entry, PlayState, SingleMode, Song, Status, Subsystem, Tag, TagFilter};
pub use spectrum::{SPECTRUM_MAX_HZ, SPECTRUM_MIN_HZ, SpectrumFrame, band_centers, band_edges};
pub use timeline::{AlbumRun, Timeline, timeline};
