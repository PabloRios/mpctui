//! Formato de audio que informa el servidor mientras hay reproducción.

/// Cómo se codifica cada muestra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// Enteros con signo de `bits` bits.
    Integer { bits: u8 },
    /// Punto flotante.
    Float,
    /// DSD; `rate` es el múltiplo de 44,1 kHz (64, 128…).
    Dsd { rate: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    /// En Hz; `None` en DSD, cuya tasa va en `SampleFormat::Dsd`.
    pub sample_rate: Option<u32>,
    pub format: SampleFormat,
    pub channels: u8,
}
