//! Log a archivo: la terminal la ocupa la TUI.

use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::project_directories;

const LOG_FILE_NAME: &str = "mpctui.log";

/// Directorio de estado (`~/.local/state/mpctui`) o, si no existe, el de datos locales.
pub fn log_directory() -> Option<PathBuf> {
    let directories = project_directories()?;
    let directory = directories
        .state_dir()
        .unwrap_or_else(|| directories.data_local_dir());
    Some(directory.to_path_buf())
}

/// Inicia el log; el guard devuelto debe vivir hasta el final del programa.
pub fn init() -> anyhow::Result<WorkerGuard> {
    let directory = log_directory().context("no se encontró un directorio para el log")?;
    fs::create_dir_all(&directory)
        .with_context(|| format!("no se pudo crear {}", directory.display()))?;
    let appender = tracing_appender::rolling::never(&directory, LOG_FILE_NAME);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_max_level(Level::INFO)
        .init();
    Ok(guard)
}
