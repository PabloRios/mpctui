//! Dominio: modelos compartidos y entradas validadas. No depende de ninguna otra capa.

mod audio;
mod inputs;
mod lyrics;
mod models;
mod spectrum;
mod timeline;
mod visual;

pub use audio::{AudioFormat, SampleFormat};
pub(crate) use inputs::PATH_SEPARATOR;
pub use inputs::{
    InputError, MAX_VOLUME, PCM_BITS, PCM_MAX_CHANNELS, PCM_MAX_SAMPLE_RATE, PCM_MIN_SAMPLE_RATE,
    Password, PcmFormat, PlaylistName, SearchText, Volume,
};
pub use lyrics::{FetchedLyrics, LyricLine, Lyrics, LyricsQuery, LyricsSource, TrackIdentity};
pub use models::{
    Entry, MoveDirection, PlayState, SingleMode, Song, Status, Subsystem, Tag, TagFilter,
};
pub use spectrum::{SPECTRUM_MAX_HZ, SPECTRUM_MIN_HZ, band_centers, band_edges};
pub use timeline::{AlbumRun, Timeline, timeline};
pub use visual::{
    BandFrame, ChannelLevel, METER_FLOOR_DB, METER_PEAK_HOLD, RMS_WINDOW, STEREO_CHANNELS,
    SampleRange, ScopePoint, VisualContent, VisualFrame, VisualRequest, VisualizerStyle,
    WATERFALL_COLUMNS_PER_SECOND,
};
