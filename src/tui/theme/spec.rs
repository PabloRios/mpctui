//! Cómo se escribe un tema: un color por rol y las paradas de cada degradado. Los
//! valores están en `palettes.rs`.

use ratatui::style::Color;

use super::roles::{GradientRole, Role};

/// Un color de 24 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    /// `0x5fb3f9` → `#5fb3f9`.
    pub const fn hex(value: u32) -> Self {
        let [_, red, green, blue] = value.to_be_bytes();
        Self { red, green, blue }
    }
}

/// Un color tal como lo define un tema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swatch {
    /// Uno de los 16 colores con nombre: toma la paleta de la terminal.
    Ansi(Color),
    Rgb(Rgb),
    /// El color de texto de la terminal (sin pintar).
    TerminalText,
}

pub struct ThemeSpec {
    pub roles: RoleSwatches,
    pub gradients: GradientSwatches,
}

/// Un color por rol. Si se agrega un rol y un tema no lo define, no compila.
pub struct RoleSwatches {
    pub text: Swatch,
    pub text_strong: Swatch,
    pub muted: Swatch,
    pub subtle: Swatch,
    pub accent: Swatch,
    pub accent_alt: Swatch,
    pub tab_active: Swatch,
    pub playing: Swatch,
    pub paused: Swatch,
    pub artist: Swatch,
    pub album: Swatch,
    pub duration: Swatch,
    pub directory: Swatch,
    pub playlist: Swatch,
    pub match_text: Swatch,
    pub selection: Swatch,
    pub success: Swatch,
    pub warning: Swatch,
    pub error: Swatch,
    pub info: Swatch,
    pub lossless: Swatch,
    pub lossy: Swatch,
    pub high_resolution: Swatch,
    pub peak: Swatch,
}

impl RoleSwatches {
    pub fn get(&self, role: Role) -> Swatch {
        match role {
            Role::Text => self.text,
            Role::TextStrong => self.text_strong,
            Role::Muted => self.muted,
            Role::Subtle => self.subtle,
            Role::Accent => self.accent,
            Role::AccentAlt => self.accent_alt,
            Role::TabActive => self.tab_active,
            Role::Playing => self.playing,
            Role::Paused => self.paused,
            Role::Artist => self.artist,
            Role::Album => self.album,
            Role::Duration => self.duration,
            Role::Directory => self.directory,
            Role::Playlist => self.playlist,
            Role::Match => self.match_text,
            Role::Selection => self.selection,
            Role::Success => self.success,
            Role::Warning => self.warning,
            Role::Error => self.error,
            Role::Info => self.info,
            Role::Lossless => self.lossless,
            Role::Lossy => self.lossy,
            Role::HighResolution => self.high_resolution,
            Role::Peak => self.peak,
        }
    }
}

/// Las paradas de cada degradado (de 1 a 5), repartidas de forma pareja entre 0 y 1.
pub struct GradientSwatches {
    pub progress: &'static [Swatch],
    pub volume: &'static [Swatch],
    pub bars: &'static [Swatch],
    pub heat: &'static [Swatch],
    pub mirror_left: &'static [Swatch],
    pub mirror_right: &'static [Swatch],
    pub wave: &'static [Swatch],
}

impl GradientSwatches {
    pub fn get(&self, role: GradientRole) -> &'static [Swatch] {
        match role {
            GradientRole::Progress => self.progress,
            GradientRole::Volume => self.volume,
            GradientRole::Bars => self.bars,
            GradientRole::Heat => self.heat,
            GradientRole::MirrorLeft => self.mirror_left,
            GradientRole::MirrorRight => self.mirror_right,
            GradientRole::Wave => self.wave,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_splits_the_channels() {
        let color = Rgb::hex(0x5f_b3_f9);
        assert_eq!((color.red, color.green, color.blue), (0x5f, 0xb3, 0xf9));
    }
}
