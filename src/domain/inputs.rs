//! Tipos de entrada validados al construirse (fail fast): si existe un valor
//! de estos tipos, ya es válido y las capas internas no vuelven a validarlo.

use std::fmt;

pub const MAX_VOLUME: u8 = 100;
pub(crate) const PATH_SEPARATOR: char = '/';
/// Única profundidad que acepta el visualizador.
pub const PCM_BITS: u8 = 16;
pub const PCM_MIN_SAMPLE_RATE: u32 = 8_000;
pub const PCM_MAX_SAMPLE_RATE: u32 = 192_000;
pub const PCM_MAX_CHANNELS: u8 = 2;
/// Separa los campos de un formato PCM: `44100:16:2`, como en `mpd.conf`.
const PCM_FIELD_SEPARATOR: char = ':';
const BITS_PER_BYTE: u8 = 8;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    #[error("el volumen debe estar entre 0 y {MAX_VOLUME}; se recibió {0}")]
    VolumeOutOfRange(u8),
    #[error("el texto no puede estar vacío")]
    EmptyText,
    #[error("el texto no puede contener saltos de línea ni caracteres de control")]
    ControlCharacter,
    #[error("el nombre de la playlist no puede contener '{PATH_SEPARATOR}'")]
    PathSeparatorInName,
    #[error("el formato PCM debe ser frecuencia:bits:canales (como \"44100:16:2\"), no {0:?}")]
    MalformedPcmFormat(String),
    #[error(
        "la frecuencia debe estar entre {PCM_MIN_SAMPLE_RATE} y {PCM_MAX_SAMPLE_RATE} Hz; se recibió {0}"
    )]
    SampleRateOutOfRange(u32),
    #[error("solo se admiten muestras de {PCM_BITS} bits; se recibió {0}")]
    UnsupportedBits(u8),
    #[error("solo se admiten 1 o {PCM_MAX_CHANNELS} canales; se recibió {0}")]
    UnsupportedChannels(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Volume(u8);

impl Volume {
    pub fn new(value: u8) -> Result<Self, InputError> {
        if value > MAX_VOLUME {
            return Err(InputError::VolumeOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> u8 {
        self.0
    }

    /// Sube o baja `delta` puntos sin salirse de `0..=MAX_VOLUME`.
    pub fn stepped(self, delta: i16) -> Self {
        let target = (i16::from(self.0) + delta).clamp(0, i16::from(MAX_VOLUME));
        Self(target as u8)
    }
}

/// Texto de búsqueda: no vacío y sin caracteres de control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText(String);

impl SearchText {
    pub fn new(raw: &str) -> Result<Self, InputError> {
        validate_single_line(raw).map(|text| Self(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Nombre de playlist guardada: una línea, no vacío y sin `/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlaylistName(String);

impl PlaylistName {
    pub fn new(raw: &str) -> Result<Self, InputError> {
        let name = validate_single_line(raw)?;
        if name.contains(PATH_SEPARATOR) {
            return Err(InputError::PathSeparatorInName);
        }
        Ok(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlaylistName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Formato del PCM que escribe la salida fifo de MPD: enteros de 16 bits con signo,
/// 1 o 2 canales, entre 8 y 192 kHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFormat {
    sample_rate: u32,
    channels: u8,
}

impl PcmFormat {
    pub fn new(sample_rate: u32, bits: u8, channels: u8) -> Result<Self, InputError> {
        if !(PCM_MIN_SAMPLE_RATE..=PCM_MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(InputError::SampleRateOutOfRange(sample_rate));
        }
        if bits != PCM_BITS {
            return Err(InputError::UnsupportedBits(bits));
        }
        if !(1..=PCM_MAX_CHANNELS).contains(&channels) {
            return Err(InputError::UnsupportedChannels(channels));
        }
        Ok(Self {
            sample_rate,
            channels,
        })
    }

    /// `frecuencia:bits:canales`, como el `format` de `mpd.conf`.
    pub fn parse(raw: &str) -> Result<Self, InputError> {
        let malformed = || InputError::MalformedPcmFormat(raw.to_owned());
        let fields: Vec<&str> = raw.trim().split(PCM_FIELD_SEPARATOR).collect();
        let [sample_rate, bits, channels] = fields.as_slice() else {
            return Err(malformed());
        };
        Self::new(
            sample_rate.parse().map_err(|_| malformed())?,
            bits.parse().map_err(|_| malformed())?,
            channels.parse().map_err(|_| malformed())?,
        )
    }

    pub fn sample_rate(self) -> u32 {
        self.sample_rate
    }

    pub fn channels(self) -> u8 {
        self.channels
    }

    /// Bytes de una muestra de un canal.
    pub fn bytes_per_sample(self) -> usize {
        usize::from(PCM_BITS / BITS_PER_BYTE)
    }

    /// Bytes de un cuadro: una muestra por canal.
    pub fn bytes_per_frame(self) -> usize {
        self.bytes_per_sample() * usize::from(self.channels)
    }
}

/// `44100:16:2`, como se escribe en `mpd.conf`.
impl fmt::Display for PcmFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{PCM_FIELD_SEPARATOR}{PCM_BITS}{PCM_FIELD_SEPARATOR}{}",
            self.sample_rate, self.channels
        )
    }
}

/// Contraseña de MPD: se respeta tal cual (sin recortar), pero en una sola línea.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    pub fn new(raw: &str) -> Result<Self, InputError> {
        reject_control_characters(raw)?;
        if raw.is_empty() {
            return Err(InputError::EmptyText);
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// No se imprime nunca el valor real.
impl fmt::Debug for Password {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Password(***)")
    }
}

/// Recorta espacios y exige texto no vacío de una sola línea.
fn validate_single_line(raw: &str) -> Result<&str, InputError> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(InputError::EmptyText);
    }
    reject_control_characters(text)?;
    Ok(text)
}

/// Un salto de línea cortaría el comando enviado a MPD.
fn reject_control_characters(text: &str) -> Result<(), InputError> {
    if text.chars().any(char::is_control) {
        return Err(InputError::ControlCharacter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_rejects_values_above_maximum() {
        assert_eq!(Volume::new(MAX_VOLUME).unwrap().value(), MAX_VOLUME);
        assert_eq!(Volume::new(101), Err(InputError::VolumeOutOfRange(101)));
    }

    #[test]
    fn volume_steps_stay_in_range() {
        let volume = Volume::new(97).unwrap();
        assert_eq!(volume.stepped(5).value(), MAX_VOLUME);
        assert_eq!(volume.stepped(-5).value(), 92);
        assert_eq!(Volume::new(3).unwrap().stepped(-5).value(), 0);
    }

    #[test]
    fn search_text_is_trimmed_and_single_line() {
        assert_eq!(SearchText::new("  love ").unwrap().as_str(), "love");
        assert_eq!(SearchText::new("   "), Err(InputError::EmptyText));
        assert_eq!(
            SearchText::new("a\nclear"),
            Err(InputError::ControlCharacter)
        );
    }

    #[test]
    fn password_keeps_spaces_but_rejects_newlines() {
        assert_eq!(Password::new(" clave ").unwrap().as_str(), " clave ");
        assert_eq!(Password::new("a\nb"), Err(InputError::ControlCharacter));
        assert_eq!(
            format!("{:?}", Password::new("x").unwrap()),
            "Password(***)"
        );
    }

    #[test]
    fn pcm_format_accepts_16_bit_mono_or_stereo() {
        let format = PcmFormat::parse("44100:16:2").unwrap();
        assert_eq!(format.sample_rate(), 44_100);
        assert_eq!(format.channels(), 2);
        assert_eq!(format.bytes_per_frame(), 4);
        assert_eq!(format.to_string(), "44100:16:2");
        assert_eq!(PcmFormat::parse("8000:16:1").unwrap().bytes_per_frame(), 2);
    }

    #[test]
    fn pcm_format_rejects_what_the_visualizer_cannot_read() {
        let malformed = |raw: &str| InputError::MalformedPcmFormat(raw.to_owned());
        assert_eq!(PcmFormat::parse("44100:16"), Err(malformed("44100:16")));
        assert_eq!(PcmFormat::parse("abc:16:2"), Err(malformed("abc:16:2")));
        assert_eq!(
            PcmFormat::parse("44100:24:2"),
            Err(InputError::UnsupportedBits(24))
        );
        assert_eq!(
            PcmFormat::parse("44100:16:6"),
            Err(InputError::UnsupportedChannels(6))
        );
        assert_eq!(
            PcmFormat::parse("4000:16:2"),
            Err(InputError::SampleRateOutOfRange(4_000))
        );
    }

    #[test]
    fn playlist_name_rejects_path_separator() {
        assert_eq!(PlaylistName::new("Rock 80s").unwrap().as_str(), "Rock 80s");
        assert_eq!(
            PlaylistName::new("rock/80s"),
            Err(InputError::PathSeparatorInName)
        );
        assert_eq!(PlaylistName::new(""), Err(InputError::EmptyText));
    }
}
