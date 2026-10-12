//! Análisis del visualizador, en funciones puras: decodificar el PCM, guardar los
//! últimos pares de muestras y un módulo por forma, más los compartidos (bandas,
//! niveles y picos retenidos).

mod analysis;
mod bands;
mod levels;
mod meters;
mod pcm;
mod peak;
mod vectorscope;
mod waterfall;
mod wave;

pub use analysis::{Analysis, SILENCE_TIMEOUT};
pub use bands::{CEIL_DB, FFT_SIZE, FLOOR_DB};
#[cfg(test)]
pub(crate) use pcm::sine_pcm;
