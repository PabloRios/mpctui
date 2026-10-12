//! Contratos angostos de acceso a datos. Los servicios dependen de estos traits,
//! nunca de la implementación concreta.

use std::future::Future;

use super::error::Result;
use crate::domain::{
    Entry, FetchedLyrics, PlaylistName, SearchText, SingleMode, Song, Status, Subsystem, Tag,
    TagFilter, TrackIdentity, Volume,
};

pub trait PlayerRepository {
    fn status(&mut self) -> impl Future<Output = Result<Status>> + Send;
    fn current_song(&mut self) -> impl Future<Output = Result<Option<Song>>> + Send;
    /// Reproduce desde `position` de la cola, o reanuda si es `None`.
    fn play(&mut self, position: Option<u32>) -> impl Future<Output = Result<()>> + Send;
    fn set_pause(&mut self, paused: bool) -> impl Future<Output = Result<()>> + Send;
    fn stop(&mut self) -> impl Future<Output = Result<()>> + Send;
    fn next(&mut self) -> impl Future<Output = Result<()>> + Send;
    fn previous(&mut self) -> impl Future<Output = Result<()>> + Send;
    fn set_volume(&mut self, volume: Volume) -> impl Future<Output = Result<()>> + Send;
    /// Desplaza el tema actual; negativo va hacia atrás.
    fn seek_relative(&mut self, offset_seconds: i32) -> impl Future<Output = Result<()>> + Send;
}

pub trait OptionsRepository {
    fn set_random(&mut self, enabled: bool) -> impl Future<Output = Result<()>> + Send;
    fn set_repeat(&mut self, enabled: bool) -> impl Future<Output = Result<()>> + Send;
    fn set_single(&mut self, mode: SingleMode) -> impl Future<Output = Result<()>> + Send;
    fn set_consume(&mut self, enabled: bool) -> impl Future<Output = Result<()>> + Send;
}

pub trait QueueRepository {
    fn queue(&mut self) -> impl Future<Output = Result<Vec<Song>>> + Send;
    /// Agrega un archivo o un directorio completo.
    fn add(&mut self, uri: &str) -> impl Future<Output = Result<()>> + Send;
    /// Agrega todas las canciones que cumplen los filtros.
    fn add_matching(&mut self, filters: &[TagFilter]) -> impl Future<Output = Result<()>> + Send;
    fn delete(&mut self, position: u32) -> impl Future<Output = Result<()>> + Send;
    fn clear(&mut self) -> impl Future<Output = Result<()>> + Send;
}

pub trait LibraryRepository {
    /// Contenido de un directorio; `""` es la raíz.
    fn browse(&mut self, path: &str) -> impl Future<Output = Result<Vec<Entry>>> + Send;
    /// Valores únicos de `tag` entre las canciones que cumplen los filtros.
    fn list_tag(
        &mut self,
        tag: Tag,
        filters: &[TagFilter],
    ) -> impl Future<Output = Result<Vec<String>>> + Send;
    /// Canciones que cumplen exactamente los filtros.
    fn find(&mut self, filters: &[TagFilter]) -> impl Future<Output = Result<Vec<Song>>> + Send;
    /// Canciones cuyo `field` contiene `text`, sin distinguir mayúsculas.
    fn search(
        &mut self,
        field: Tag,
        text: &SearchText,
    ) -> impl Future<Output = Result<Vec<Song>>> + Send;
}

pub trait PlaylistRepository {
    fn playlists(&mut self) -> impl Future<Output = Result<Vec<PlaylistName>>> + Send;
    fn playlist_songs(
        &mut self,
        name: &PlaylistName,
    ) -> impl Future<Output = Result<Vec<Song>>> + Send;
    /// Agrega la playlist al final de la cola.
    fn load(&mut self, name: &PlaylistName) -> impl Future<Output = Result<()>> + Send;
    /// Guarda la cola como playlist nueva.
    fn save(&mut self, name: &PlaylistName) -> impl Future<Output = Result<()>> + Send;
    fn remove(&mut self, name: &PlaylistName) -> impl Future<Output = Result<()>> + Send;
}

pub trait ChangesRepository {
    /// Bloquea hasta que cambie algo en el servidor y devuelve qué cambió.
    fn wait_for_changes(&mut self) -> impl Future<Output = Result<Vec<Subsystem>>> + Send;
    /// Confirma que el servidor responde (reconectando si hace falta). `idle` no sirve
    /// para eso: si la conexión está bien, bloquea hasta el próximo cambio.
    fn ping(&mut self) -> impl Future<Output = Result<()>> + Send;
}

/// Fuente de PCM crudo (la salida fifo de MPD). Solo lectura.
pub trait PcmSource {
    /// Lee lo que haya disponible en `destination`. `Ok(0)` es fin de archivo: el
    /// escritor cerró su lado; la próxima lectura vuelve a abrir la fuente.
    fn read(&mut self, destination: &mut [u8]) -> impl Future<Output = Result<usize>> + Send;
}

/// La letra embebida en los tags del archivo (`readcomments`).
pub trait EmbeddedLyricsRepository {
    fn embedded_lyrics(&mut self, uri: &str)
    -> impl Future<Output = Result<Option<String>>> + Send;
}

/// Letras en archivos locales: `.lrc` primero y, si no, `.txt`.
pub trait LyricsFiles {
    /// La de la caché para ese tema.
    fn cached(
        &mut self,
        track: &TrackIdentity,
    ) -> impl Future<Output = Result<Option<String>>> + Send;
    /// La que está junto al archivo del tema, con su mismo nombre.
    fn beside_song(&mut self, uri: &str) -> impl Future<Output = Result<Option<String>>> + Send;
    /// Guarda en la caché: `.lrc` si es sincronizada, `.txt` si no.
    fn save(
        &mut self,
        track: &TrackIdentity,
        text: &str,
        synced: bool,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Letras de un servicio externo (LRCLIB).
pub trait LyricsProvider {
    fn fetch(
        &mut self,
        track: &TrackIdentity,
    ) -> impl Future<Output = Result<FetchedLyrics>> + Send;
}
