//! Roles de color: lo que piden las vistas. Cada tema dice de qué color es cada rol; los
//! modificadores son parte del rol, así se mantienen sin color.

use ratatui::style::Modifier;

/// Un tipo de dato o de elemento de la interfaz con su propio color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Texto común: queda con el color de la terminal.
    Text,
    /// Título del tema en el panel, nombre de la forma del visualizador.
    TextStrong,
    /// Texto secundario: posiciones, etiquetas, descripciones de atajos, contadores.
    Muted,
    /// Bordes, reglas, partes vacías de barras. Decorativo: se aparta a propósito.
    Subtle,
    /// Teclas, cursor, nombre de la aplicación, panel con foco.
    Accent,
    /// Cabeza de las barras de progreso, títulos de sección de la ayuda.
    AccentAlt,
    /// Pestaña activa y título de la columna o panel con foco.
    TabActive,
    Playing,
    Paused,
    Artist,
    Album,
    Duration,
    Directory,
    Playlist,
    /// Texto que coincide con la búsqueda.
    Match,
    /// Fondo de la fila seleccionada; sin colores de tema, video inverso.
    Selection,
    Success,
    Warning,
    Error,
    Info,
    /// Formato de audio sin pérdida.
    Lossless,
    /// Formato de audio con pérdida.
    Lossy,
    /// Frecuencia y bits de un formato sin pérdida de alta resolución.
    HighResolution,
    /// Picos del visualizador.
    Peak,
}

impl Role {
    pub const ALL: [Role; 24] = [
        Role::Text,
        Role::TextStrong,
        Role::Muted,
        Role::Subtle,
        Role::Accent,
        Role::AccentAlt,
        Role::TabActive,
        Role::Playing,
        Role::Paused,
        Role::Artist,
        Role::Album,
        Role::Duration,
        Role::Directory,
        Role::Playlist,
        Role::Match,
        Role::Selection,
        Role::Success,
        Role::Warning,
        Role::Error,
        Role::Info,
        Role::Lossless,
        Role::Lossy,
        Role::HighResolution,
        Role::Peak,
    ];
    pub const COUNT: usize = Self::ALL.len();

    /// Posición en `ALL`, para las tablas indexadas por rol.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Lo que distingue al rol aunque no haya color.
    pub fn modifier(self) -> Modifier {
        match self {
            Role::TextStrong
            | Role::AccentAlt
            | Role::Playing
            | Role::Paused
            | Role::Directory
            | Role::Error
            | Role::Lossless
            | Role::Lossy => Modifier::BOLD,
            Role::TabActive => Modifier::BOLD | Modifier::REVERSED,
            Role::Match => Modifier::REVERSED | Modifier::UNDERLINED,
            Role::Selection => Modifier::REVERSED,
            Role::Text
            | Role::Muted
            | Role::Subtle
            | Role::Accent
            | Role::Artist
            | Role::Album
            | Role::Duration
            | Role::Playlist
            | Role::Success
            | Role::Warning
            | Role::Info
            | Role::HighResolution
            | Role::Peak => Modifier::empty(),
        }
    }
}

/// Degradados: el color de una celda sale de su posición entre 0 y 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientRole {
    /// Barras de progreso y duración relativa.
    Progress,
    /// Volumen y Medidores: de verde a rojo hacia el máximo.
    Volume,
    /// Barras del visualizador, por altura.
    Bars,
    /// Intensidad de la Cascada y densidad del Vectorscopio.
    Heat,
    MirrorLeft,
    MirrorRight,
    /// Onda, por amplitud.
    Wave,
}

impl GradientRole {
    pub const ALL: [GradientRole; 7] = [
        GradientRole::Progress,
        GradientRole::Volume,
        GradientRole::Bars,
        GradientRole::Heat,
        GradientRole::MirrorLeft,
        GradientRole::MirrorRight,
        GradientRole::Wave,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub fn index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_follow_the_order_of_all() {
        for (position, role) in Role::ALL.into_iter().enumerate() {
            assert_eq!(role.index(), position, "{role:?}");
        }
        for (position, role) in GradientRole::ALL.into_iter().enumerate() {
            assert_eq!(role.index(), position, "{role:?}");
        }
    }

    #[test]
    fn states_keep_a_modifier_without_color() {
        for role in [Role::TabActive, Role::Match, Role::Selection, Role::Error] {
            assert!(!role.modifier().is_empty(), "{role:?}");
        }
        assert!(Role::TabActive.modifier().contains(Modifier::REVERSED));
        assert!(Role::Text.modifier().is_empty());
    }
}
