//! Letras en archivos locales: la caché (`~/.lyrics/Artista - Título.lrc`, como
//! ncmpcpp y ncmpc) y la carpeta del tema (`tema.lrc` junto a `tema.flac`). Un archivo
//! que no existe no es un error: simplemente no hay letra ahí.

use std::io;
use std::path::{Component, Path, PathBuf};

use super::error::Result;
use super::traits::LyricsFiles;
use crate::domain::{PATH_SEPARATOR, TrackIdentity};

const SYNCED_EXTENSION: &str = "lrc";
const PLAIN_EXTENSION: &str = "txt";
/// Extensiones que se prueban, en orden: la sincronizada primero.
const EXTENSIONS: [&str; 2] = [SYNCED_EXTENSION, PLAIN_EXTENSION];
/// `Artista - Título`.
const NAME_SEPARATOR: &str = " - ";
/// Reemplaza lo que no puede ir en un nombre de archivo.
const UNSAFE_REPLACEMENT: char = '_';
/// Los streams (`http://…`) no tienen carpeta.
const URL_SCHEME_SEPARATOR: &str = "://";
/// Algunos editores guardan el texto con una marca de orden de bytes al principio.
const BYTE_ORDER_MARK: char = '\u{feff}';

pub struct LocalLyricsFiles {
    cache_directory: PathBuf,
    /// La raíz de la biblioteca de MPD; sin ella no se busca junto al tema.
    music_directory: Option<PathBuf>,
}

impl LocalLyricsFiles {
    pub fn new(cache_directory: PathBuf, music_directory: Option<PathBuf>) -> Self {
        Self {
            cache_directory,
            music_directory,
        }
    }
}

impl LyricsFiles for LocalLyricsFiles {
    async fn cached(&mut self, track: &TrackIdentity) -> Result<Option<String>> {
        let stem = cache_file_stem(track);
        let candidates =
            EXTENSIONS.map(|extension| self.cache_directory.join(format!("{stem}.{extension}")));
        read_first(&candidates).await
    }

    async fn beside_song(&mut self, uri: &str) -> Result<Option<String>> {
        let (Some(music), Some(relative)) = (&self.music_directory, library_path(uri)) else {
            return Ok(None);
        };
        let song = music.join(relative);
        let candidates = EXTENSIONS.map(|extension| song.with_extension(extension));
        read_first(&candidates).await
    }

    async fn save(&mut self, track: &TrackIdentity, text: &str, synced: bool) -> Result<()> {
        let extension = if synced {
            SYNCED_EXTENSION
        } else {
            PLAIN_EXTENSION
        };
        tokio::fs::create_dir_all(&self.cache_directory).await?;
        let path = self
            .cache_directory
            .join(format!("{}.{extension}", cache_file_stem(track)));
        tokio::fs::write(path, text).await?;
        Ok(())
    }
}

/// `Artista - Título`, sin separadores de ruta.
fn cache_file_stem(track: &TrackIdentity) -> String {
    format!(
        "{}{NAME_SEPARATOR}{}",
        safe_name(track.artist()),
        safe_name(track.title())
    )
}

fn safe_name(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character == PATH_SEPARATOR {
                UNSAFE_REPLACEMENT
            } else {
                character
            }
        })
        .collect()
}

/// La ruta del tema dentro de la biblioteca; `None` para streams o rutas que salen de
/// ella (`..`, absolutas).
fn library_path(uri: &str) -> Option<&Path> {
    if uri.contains(URL_SCHEME_SEPARATOR) {
        return None;
    }
    let path = Path::new(uri);
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then_some(path)
}

/// El contenido del primer archivo que exista.
async fn read_first(candidates: &[PathBuf]) -> Result<Option<String>> {
    for path in candidates {
        match tokio::fs::read(path).await {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                return Ok(Some(text.trim_start_matches(BYTE_ORDER_MARK).to_owned()));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    /// Directorio temporal propio de cada test, borrado al terminar.
    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("mpctui-{name}-{nanos}"));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn track(artist: &str, title: &str) -> TrackIdentity {
        TrackIdentity::new(artist, title, None, None).unwrap()
    }

    #[test]
    fn cache_names_have_no_path_separators() {
        assert_eq!(cache_file_stem(&track("AC/DC", "T.N.T.")), "AC_DC - T.N.T.");
    }

    #[test]
    fn only_paths_inside_the_library_are_used() {
        assert_eq!(library_path("Rock/a.flac"), Some(Path::new("Rock/a.flac")));
        assert_eq!(library_path("../a.flac"), None);
        assert_eq!(library_path("/etc/a.flac"), None);
        assert_eq!(library_path("http://radio/a.mp3"), None);
    }

    #[tokio::test]
    async fn the_cache_prefers_synced_lyrics() {
        let directory = TemporaryDirectory::new("cache");
        let mut files = LocalLyricsFiles::new(directory.0.clone(), None);
        let time = track("Pink Floyd", "Time");
        assert_eq!(files.cached(&time).await.unwrap(), None);
        std::fs::write(directory.0.join("Pink Floyd - Time.txt"), "texto").unwrap();
        assert_eq!(files.cached(&time).await.unwrap().as_deref(), Some("texto"));
        std::fs::write(
            directory.0.join("Pink Floyd - Time.lrc"),
            "\u{feff}[00:01]x",
        )
        .unwrap();
        assert_eq!(
            files.cached(&time).await.unwrap().as_deref(),
            Some("[00:01]x")
        );
    }

    #[tokio::test]
    async fn saved_lyrics_are_read_back_from_the_cache() {
        let directory = TemporaryDirectory::new("save");
        let cache = directory.0.join("letras");
        let mut files = LocalLyricsFiles::new(cache.clone(), None);
        let time = track("Pink Floyd", "Time");
        files.save(&time, "[00:01]x", true).await.unwrap();
        assert!(cache.join("Pink Floyd - Time.lrc").exists());
        assert_eq!(
            files.cached(&time).await.unwrap().as_deref(),
            Some("[00:01]x")
        );
        let other = track("Otro", "Tema");
        files.save(&other, "texto", false).await.unwrap();
        assert!(cache.join("Otro - Tema.txt").exists());
    }

    #[tokio::test]
    async fn lyrics_beside_the_song_share_its_name() {
        let directory = TemporaryDirectory::new("music");
        std::fs::create_dir_all(directory.0.join("Rock")).unwrap();
        std::fs::write(directory.0.join("Rock/01 Time.lrc"), "[00:01]x").unwrap();
        let mut files = LocalLyricsFiles::new(directory.0.join("cache"), Some(directory.0.clone()));
        let found = files.beside_song("Rock/01 Time.flac").await.unwrap();
        assert_eq!(found.as_deref(), Some("[00:01]x"));
        assert_eq!(files.beside_song("Rock/02 Otro.flac").await.unwrap(), None);
        let mut without_music = LocalLyricsFiles::new(directory.0.clone(), None);
        assert_eq!(
            without_music
                .beside_song("Rock/01 Time.flac")
                .await
                .unwrap(),
            None
        );
    }
}
