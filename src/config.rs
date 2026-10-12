//! Configuración: archivo TOML + variables de entorno, validada al cargar (fail fast).

use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs, io};

use directories::ProjectDirs;
use serde::Deserialize;

use crate::domain::{InputError, Password, PcmFormat, VisualizerStyle};
use crate::repository::mpd::{Address, DEFAULT_HOST, DEFAULT_PORT};

const APPLICATION_NAME: &str = "mpctui";
const CONFIG_FILE_NAME: &str = "config.toml";
const HOST_VARIABLE: &str = "MPD_HOST";
const PORT_VARIABLE: &str = "MPD_PORT";
const GLYPHS_VARIABLE: &str = "MPCTUI_GLYPHS";
/// Variables de locale, en orden de prioridad.
const LOCALE_VARIABLES: [&str; 3] = ["LC_ALL", "LC_CTYPE", "LANG"];
const TERMINAL_VARIABLE: &str = "TERM";
/// Marcas de un locale UTF-8, en minúsculas.
const UTF8_MARKERS: [&str; 2] = ["utf-8", "utf8"];
/// Terminales sin los símbolos Unicode del diseño: la consola virtual y la "tonta".
const ASCII_TERMINALS: [&str; 2] = ["linux", "dumb"];
/// Salida fifo de MPD que lee el visualizador, y su formato (el `format` de `mpd.conf`).
const DEFAULT_VISUALIZER_FIFO: &str = "/tmp/mpd.fifo";
const DEFAULT_VISUALIZER_FORMAT: &str = "44100:16:2";
/// Nombres de `visualizer_style`, en inglés como los de `glyphs`.
const VISUALIZER_STYLE_NAMES: [(&str, VisualizerStyle); 6] = [
    ("bars", VisualizerStyle::Bars),
    ("mirror", VisualizerStyle::Mirror),
    ("wave", VisualizerStyle::Wave),
    ("waterfall", VisualizerStyle::Waterfall),
    ("vectorscope", VisualizerStyle::Vectorscope),
    ("meters", VisualizerStyle::Meters),
];
/// Como en `mpc`: `MPD_HOST=contraseña@host`.
const PASSWORD_SEPARATOR: char = '@';

/// Juego de glifos pedido por el usuario.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum GlyphSetting {
    /// Se decide según el locale y la terminal.
    #[default]
    Auto,
    Unicode,
    Ascii,
}

impl GlyphSetting {
    const AUTO: &str = "auto";
    const UNICODE: &str = "unicode";
    const ASCII: &str = "ascii";
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("glyphs debe ser \"{auto}\", \"{unicode}\" o \"{ascii}\", no {0:?}",
    auto = GlyphSetting::AUTO, unicode = GlyphSetting::UNICODE, ascii = GlyphSetting::ASCII)]
pub struct InvalidGlyphSetting(String);

impl FromStr for GlyphSetting {
    type Err = InvalidGlyphSetting;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::AUTO => Ok(Self::Auto),
            Self::UNICODE => Ok(Self::Unicode),
            Self::ASCII => Ok(Self::Ascii),
            other => Err(InvalidGlyphSetting(other.to_owned())),
        }
    }
}

impl TryFrom<String> for GlyphSetting {
    type Error = InvalidGlyphSetting;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Juego de glifos ya resuelto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphMode {
    Unicode,
    Ascii,
}

/// Contenido del archivo de configuración; todos los campos son opcionales.
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub glyphs: Option<GlyphSetting>,
    /// Barra de progreso del álbum en el panel; oculta por defecto.
    pub album_progress: Option<bool>,
    /// Barra de progreso de la cola en el panel; oculta por defecto.
    pub queue_progress: Option<bool>,
    /// Ruta de la salida fifo de MPD para el visualizador.
    pub visualizer_fifo: Option<String>,
    /// Formato de esa salida: `frecuencia:16:canales`.
    pub visualizer_format: Option<String>,
    /// Forma con la que arranca el visualizador (`bars`, `mirror`, `wave`…).
    pub visualizer_style: Option<String>,
}

/// Variables de entorno relevantes, leídas sin interpretar.
#[derive(Debug, Default)]
pub struct Environment {
    pub host: Option<String>,
    pub port: Option<String>,
    pub glyphs: Option<String>,
    pub terminal: TerminalEnvironment,
}

/// Lo que dice el entorno sobre la terminal, para resolver `glyphs = "auto"`.
#[derive(Debug, Default)]
pub struct TerminalEnvironment {
    /// Valores de `LC_ALL`, `LC_CTYPE` y `LANG`, en ese orden.
    pub locale: [Option<String>; LOCALE_VARIABLES.len()],
    pub term: Option<String>,
}

/// Configuración ya validada.
#[derive(Debug)]
pub struct Settings {
    pub address: Address,
    pub password: Option<Password>,
    pub glyph_mode: GlyphMode,
    pub album_progress: bool,
    pub queue_progress: bool,
    pub visualizer_fifo: PathBuf,
    pub visualizer_format: PcmFormat,
    pub visualizer_style: VisualizerStyle,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("no se pudo leer {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("configuración inválida en {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("{PORT_VARIABLE} no es un puerto válido: {0:?}")]
    InvalidPort(String),
    #[error("el host de MPD no puede estar vacío")]
    EmptyHost,
    #[error("contraseña de MPD inválida: {0}")]
    InvalidPassword(InputError),
    #[error("{GLYPHS_VARIABLE} inválida: {0}")]
    InvalidGlyphs(InvalidGlyphSetting),
    #[error("visualizer_fifo no puede estar vacío")]
    EmptyVisualizerFifo,
    #[error("visualizer_format inválido: {0}")]
    InvalidVisualizerFormat(InputError),
    #[error("visualizer_style debe ser {choices}, no {0:?}", choices = visualizer_style_choices())]
    InvalidVisualizerStyle(String),
    #[error("visualizer_style = {0:?} necesita dos canales y visualizer_format tiene uno")]
    StereoStyleWithMonoFifo(String),
}

pub(crate) fn project_directories() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", APPLICATION_NAME)
}

pub fn config_path() -> Option<PathBuf> {
    project_directories().map(|directories| directories.config_dir().join(CONFIG_FILE_NAME))
}

/// Lee archivo y entorno y los valida.
pub fn load() -> Result<Settings, ConfigError> {
    let environment = Environment {
        host: env::var(HOST_VARIABLE).ok(),
        port: env::var(PORT_VARIABLE).ok(),
        glyphs: env::var(GLYPHS_VARIABLE).ok(),
        terminal: TerminalEnvironment {
            locale: LOCALE_VARIABLES.map(|name| env::var(name).ok()),
            term: env::var(TERMINAL_VARIABLE).ok(),
        },
    };
    resolve(read_file_config()?, environment)
}

/// Precedencia: valores por defecto < archivo < entorno.
pub fn resolve(file: FileConfig, environment: Environment) -> Result<Settings, ConfigError> {
    let (environment_password, environment_host) = match environment.host.as_deref() {
        Some(value) => split_host_variable(value),
        None => (None, None),
    };
    let host = environment_host
        .or(file.host.as_deref())
        .unwrap_or(DEFAULT_HOST);
    if host.trim().is_empty() {
        return Err(ConfigError::EmptyHost);
    }
    let port = match environment.port {
        Some(value) => value.parse().map_err(|_| ConfigError::InvalidPort(value))?,
        None => file.port.unwrap_or(DEFAULT_PORT),
    };
    let password = environment_password
        .or(file.password.as_deref())
        .map(Password::new)
        .transpose()
        .map_err(ConfigError::InvalidPassword)?;
    let glyph_setting = match environment.glyphs {
        Some(value) => value.parse().map_err(ConfigError::InvalidGlyphs)?,
        None => file.glyphs.unwrap_or_default(),
    };
    let visualizer_fifo = file
        .visualizer_fifo
        .unwrap_or_else(|| DEFAULT_VISUALIZER_FIFO.to_owned());
    if visualizer_fifo.trim().is_empty() {
        return Err(ConfigError::EmptyVisualizerFifo);
    }
    let visualizer_format = PcmFormat::parse(
        file.visualizer_format
            .as_deref()
            .unwrap_or(DEFAULT_VISUALIZER_FORMAT),
    )
    .map_err(ConfigError::InvalidVisualizerFormat)?;
    let visualizer_style = match file.visualizer_style {
        Some(name) => visualizer_style(&name, visualizer_format)?,
        None => VisualizerStyle::default(),
    };
    Ok(Settings {
        address: Address::new(host, port),
        password,
        glyph_mode: resolve_glyph_mode(glyph_setting, &environment.terminal),
        album_progress: file.album_progress.unwrap_or_default(),
        queue_progress: file.queue_progress.unwrap_or_default(),
        visualizer_fifo: PathBuf::from(visualizer_fifo),
        visualizer_format,
        visualizer_style,
    })
}

/// La forma por su nombre; una estéreo con un FIFO de un canal no se puede mostrar.
fn visualizer_style(name: &str, format: PcmFormat) -> Result<VisualizerStyle, ConfigError> {
    let style = VISUALIZER_STYLE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, style)| *style)
        .ok_or_else(|| ConfigError::InvalidVisualizerStyle(name.to_owned()))?;
    if !style.is_available(format.channels()) {
        return Err(ConfigError::StereoStyleWithMonoFifo(name.to_owned()));
    }
    Ok(style)
}

/// `"bars", "mirror", …`, para el mensaje de error.
fn visualizer_style_choices() -> String {
    VISUALIZER_STYLE_NAMES
        .iter()
        .map(|(name, _)| format!("{name:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `auto` elige Unicode salvo que el locale no sea UTF-8 o la terminal no tenga los
/// símbolos (consola virtual de Linux, `dumb`).
pub fn resolve_glyph_mode(setting: GlyphSetting, terminal: &TerminalEnvironment) -> GlyphMode {
    match setting {
        GlyphSetting::Unicode => GlyphMode::Unicode,
        GlyphSetting::Ascii => GlyphMode::Ascii,
        GlyphSetting::Auto if is_utf8_locale(terminal) && !is_ascii_terminal(terminal) => {
            GlyphMode::Unicode
        }
        GlyphSetting::Auto => GlyphMode::Ascii,
    }
}

/// El locale efectivo es el primer valor no vacío de `LC_ALL`, `LC_CTYPE` y `LANG`.
fn is_utf8_locale(terminal: &TerminalEnvironment) -> bool {
    let effective = terminal
        .locale
        .iter()
        .flatten()
        .find(|value| !value.is_empty());
    effective.is_some_and(|locale| {
        let locale = locale.to_lowercase();
        UTF8_MARKERS.iter().any(|marker| locale.contains(marker))
    })
}

fn is_ascii_terminal(terminal: &TerminalEnvironment) -> bool {
    terminal
        .term
        .as_deref()
        .is_some_and(|term| ASCII_TERMINALS.contains(&term))
}

fn read_file_config() -> Result<FileConfig, ConfigError> {
    let Some(path) = config_path() else {
        return Ok(FileConfig::default());
    };
    match fs::read_to_string(&path) {
        Ok(contents) => {
            toml::from_str(&contents).map_err(|source| ConfigError::Parse { path, source })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(FileConfig::default()),
        Err(source) => Err(ConfigError::Read { path, source }),
    }
}

/// Devuelve `(contraseña, host)` a partir de `MPD_HOST`.
fn split_host_variable(value: &str) -> (Option<&str>, Option<&str>) {
    match value.split_once(PASSWORD_SEPARATOR) {
        Some((password, host)) if !password.is_empty() => (Some(password), Some(host)),
        _ => (None, Some(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(host: Option<&str>, port: Option<&str>) -> Environment {
        Environment {
            host: host.map(str::to_owned),
            port: port.map(str::to_owned),
            ..Environment::default()
        }
    }

    /// `locale` son `LC_ALL`, `LC_CTYPE` y `LANG`.
    fn terminal(
        locale: [Option<&str>; LOCALE_VARIABLES.len()],
        term: Option<&str>,
    ) -> TerminalEnvironment {
        TerminalEnvironment {
            locale: locale.map(|value| value.map(str::to_owned)),
            term: term.map(str::to_owned),
        }
    }

    fn auto_mode(locale: [Option<&str>; LOCALE_VARIABLES.len()], term: Option<&str>) -> GlyphMode {
        resolve_glyph_mode(GlyphSetting::Auto, &terminal(locale, term))
    }

    #[test]
    fn defaults_when_nothing_is_configured() {
        let settings = resolve(FileConfig::default(), Environment::default()).unwrap();
        assert_eq!(settings.address, Address::new(DEFAULT_HOST, DEFAULT_PORT));
        assert!(settings.password.is_none());
    }

    #[test]
    fn environment_overrides_file() {
        let file = FileConfig {
            host: Some("servidor".into()),
            port: Some(6601),
            password: Some("del-archivo".into()),
            glyphs: Some(GlyphSetting::Unicode),
            ..FileConfig::default()
        };
        let settings = resolve(file, environment(Some("clave@otro"), Some("6602"))).unwrap();
        assert_eq!(settings.address, Address::new("otro", 6602));
        assert_eq!(settings.password.unwrap().as_str(), "clave");
    }

    #[test]
    fn file_values_apply_without_environment() {
        let file: FileConfig =
            toml::from_str("host = \"/run/mpd/socket\"\npassword = \"x\"").unwrap();
        let settings = resolve(file, Environment::default()).unwrap();
        assert_eq!(settings.address, Address::Unix("/run/mpd/socket".into()));
        assert_eq!(settings.password.unwrap().as_str(), "x");
    }

    #[test]
    fn invalid_values_fail_fast() {
        assert!(matches!(
            resolve(FileConfig::default(), environment(None, Some("seis"))),
            Err(ConfigError::InvalidPort(_))
        ));
        assert!(matches!(
            resolve(FileConfig::default(), environment(Some(" "), None)),
            Err(ConfigError::EmptyHost)
        ));
        let file = FileConfig {
            password: Some("a\nb".into()),
            ..FileConfig::default()
        };
        assert!(matches!(
            resolve(file, Environment::default()),
            Err(ConfigError::InvalidPassword(_))
        ));
        assert!(toml::from_str::<FileConfig>("hots = \"x\"").is_err());
    }

    #[test]
    fn auto_glyphs_follow_the_first_non_empty_locale() {
        let utf8 = Some("es_AR.UTF-8");
        let term = Some("xterm-256color");
        assert_eq!(auto_mode([Some(""), None, utf8], term), GlyphMode::Unicode);
        assert_eq!(
            auto_mode([None, Some("C.utf8"), None], term),
            GlyphMode::Unicode
        );
        assert_eq!(auto_mode([None, None, Some("C")], term), GlyphMode::Ascii);
        assert_eq!(auto_mode([Some("C"), None, utf8], term), GlyphMode::Ascii);
        assert_eq!(auto_mode([None, None, None], term), GlyphMode::Ascii);
    }

    #[test]
    fn auto_glyphs_avoid_limited_terminals() {
        let utf8 = Some("es_AR.UTF-8");
        assert_eq!(
            auto_mode([None, None, utf8], Some("linux")),
            GlyphMode::Ascii
        );
        assert_eq!(
            auto_mode([None, None, utf8], Some("dumb")),
            GlyphMode::Ascii
        );
        assert_eq!(auto_mode([None, None, utf8], None), GlyphMode::Unicode);
    }

    #[test]
    fn explicit_glyphs_ignore_the_terminal() {
        let console = terminal([None, None, Some("C")], Some("linux"));
        let unicode = resolve_glyph_mode(GlyphSetting::Unicode, &console);
        assert_eq!(unicode, GlyphMode::Unicode);
        let file: FileConfig = toml::from_str("glyphs = \"ascii\"").unwrap();
        let mut environment = environment(None, None);
        environment.terminal = terminal([None, None, Some("es_AR.UTF-8")], None);
        let settings = resolve(file, environment).unwrap();
        assert_eq!(settings.glyph_mode, GlyphMode::Ascii);
    }

    #[test]
    fn environment_glyphs_override_file() {
        let file: FileConfig = toml::from_str("glyphs = \"unicode\"").unwrap();
        let mut environment = environment(None, None);
        environment.glyphs = Some("ascii".into());
        assert_eq!(
            resolve(file, environment).unwrap().glyph_mode,
            GlyphMode::Ascii
        );
    }

    #[test]
    fn album_and_queue_progress_are_hidden_by_default() {
        let settings = resolve(FileConfig::default(), Environment::default()).unwrap();
        assert!(!settings.album_progress && !settings.queue_progress);
        let file: FileConfig = toml::from_str("album_progress = true").unwrap();
        let settings = resolve(file, Environment::default()).unwrap();
        assert!(settings.album_progress && !settings.queue_progress);
        assert!(toml::from_str::<FileConfig>("queue_progress = \"si\"").is_err());
    }

    #[test]
    fn visualizer_defaults_to_the_usual_fifo() {
        let settings = resolve(FileConfig::default(), Environment::default()).unwrap();
        assert_eq!(settings.visualizer_fifo, PathBuf::from("/tmp/mpd.fifo"));
        assert_eq!(settings.visualizer_format.sample_rate(), 44_100);
        let file: FileConfig =
            toml::from_str("visualizer_fifo = \"/run/a.fifo\"\nvisualizer_format = \"48000:16:1\"")
                .unwrap();
        let settings = resolve(file, Environment::default()).unwrap();
        assert_eq!(settings.visualizer_fifo, PathBuf::from("/run/a.fifo"));
        assert_eq!(settings.visualizer_format.channels(), 1);
    }

    #[test]
    fn invalid_visualizer_settings_fail_fast() {
        let resolve_toml =
            |raw: &str| resolve(toml::from_str(raw).unwrap(), Environment::default());
        assert!(matches!(
            resolve_toml("visualizer_fifo = \" \""),
            Err(ConfigError::EmptyVisualizerFifo)
        ));
        assert!(matches!(
            resolve_toml("visualizer_format = \"44100:24:2\""),
            Err(ConfigError::InvalidVisualizerFormat(
                InputError::UnsupportedBits(24)
            ))
        ));
    }

    #[test]
    fn visualizer_style_defaults_to_bars() {
        let settings = resolve(FileConfig::default(), Environment::default()).unwrap();
        assert_eq!(settings.visualizer_style, VisualizerStyle::Bars);
        let file: FileConfig = toml::from_str("visualizer_style = \"waterfall\"").unwrap();
        let settings = resolve(file, Environment::default()).unwrap();
        assert_eq!(settings.visualizer_style, VisualizerStyle::Waterfall);
    }

    #[test]
    fn invalid_visualizer_styles_fail_fast() {
        let resolve_toml =
            |raw: &str| resolve(toml::from_str(raw).unwrap(), Environment::default());
        let error = resolve_toml("visualizer_style = \"radial\"").unwrap_err();
        assert!(matches!(error, ConfigError::InvalidVisualizerStyle(_)));
        assert!(error.to_string().contains("\"meters\""), "{error}");
        assert!(matches!(
            resolve_toml("visualizer_format = \"44100:16:1\"\nvisualizer_style = \"mirror\""),
            Err(ConfigError::StereoStyleWithMonoFifo(_))
        ));
        let mono_meters =
            resolve_toml("visualizer_format = \"44100:16:1\"\nvisualizer_style = \"meters\"");
        assert_eq!(
            mono_meters.unwrap().visualizer_style,
            VisualizerStyle::Meters
        );
    }

    #[test]
    fn invalid_glyphs_fail_fast() {
        assert!(toml::from_str::<FileConfig>("glyphs = \"emoji\"").is_err());
        let mut environment = environment(None, None);
        environment.glyphs = Some("Unicode".into());
        assert!(matches!(
            resolve(FileConfig::default(), environment),
            Err(ConfigError::InvalidGlyphs(_))
        ));
    }
}
