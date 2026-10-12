//! Búsqueda de letras: prueba las fuentes en el orden configurado y gana la primera
//! que tenga letra. La búsqueda corre en su propia tarea (`LyricsLookup`) para no
//! frenar la interfaz; si mientras tanto cambia el tema, se atiende solo el último
//! pedido.

use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use super::lrc::parse_lrc;
use crate::domain::{Lyrics, LyricsQuery, LyricsSource};
use crate::repository::{EmbeddedLyricsRepository, LyricsFiles, Result};

/// Resultado de una búsqueda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricsOutcome {
    Found(Lyrics),
    /// Ninguna fuente tenía letra. `problem` es el último error, si alguna falló.
    NotFound {
        problem: Option<String>,
    },
}

pub struct LyricsService<E, F> {
    embedded: E,
    files: F,
    /// En orden de prioridad, sin repetidos (lo valida la configuración).
    sources: Vec<LyricsSource>,
}

impl<E, F> LyricsService<E, F>
where
    E: EmbeddedLyricsRepository + Send,
    F: LyricsFiles + Send,
{
    pub fn new(embedded: E, files: F, sources: Vec<LyricsSource>) -> Self {
        Self {
            embedded,
            files,
            sources,
        }
    }

    /// Una fuente que falla no corta la búsqueda: se registra y se sigue con la próxima.
    pub async fn find(&mut self, query: &LyricsQuery) -> LyricsOutcome {
        let mut problem = None;
        for source in self.sources.clone() {
            match self.read(source, query).await {
                Ok(Some(text)) => {
                    let lyrics = parse_lrc(&text, source);
                    if !lyrics.lines.is_empty() {
                        return LyricsOutcome::Found(lyrics);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!("no se pudo leer la letra ({source:?}): {error}");
                    problem = Some(error.to_string());
                }
            }
        }
        LyricsOutcome::NotFound { problem }
    }

    /// La caché necesita artista y título.
    async fn read(&mut self, source: LyricsSource, query: &LyricsQuery) -> Result<Option<String>> {
        match (source, &query.track) {
            (LyricsSource::Cache, Some(track)) => self.files.cached(track).await,
            (LyricsSource::SongFolder, _) => self.files.beside_song(&query.uri).await,
            (LyricsSource::Tags, _) => self.embedded.embedded_lyrics(&query.uri).await,
            // LRCLIB se agrega en la fase L2.
            (LyricsSource::Cache | LyricsSource::Lrclib, _) => Ok(None),
        }
    }
}

/// Lo que encontró la búsqueda de un tema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricsResult {
    pub query: LyricsQuery,
    pub outcome: LyricsOutcome,
}

/// La tarea que busca letras. Al soltarla, se cancela.
pub struct LyricsLookup {
    task: JoinHandle<()>,
    requests: watch::Sender<Option<LyricsQuery>>,
    results: mpsc::UnboundedReceiver<LyricsResult>,
}

impl LyricsLookup {
    pub fn spawn<E, F>(service: LyricsService<E, F>) -> Self
    where
        E: EmbeddedLyricsRepository + Send + 'static,
        F: LyricsFiles + Send + 'static,
    {
        let (requests, request_receiver) = watch::channel(None);
        let (result_sender, results) = mpsc::unbounded_channel();
        let task = tokio::spawn(serve(service, request_receiver, result_sender));
        Self {
            task,
            requests,
            results,
        }
    }

    /// Reemplaza al pedido anterior si todavía no se empezó a buscar.
    pub fn request(&self, query: LyricsQuery) {
        self.requests.send_replace(Some(query));
    }

    /// Espera el próximo resultado; `None` si la tarea terminó.
    pub async fn next_result(&mut self) -> Option<LyricsResult> {
        self.results.recv().await
    }
}

impl Drop for LyricsLookup {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Una búsqueda empezada no se interrumpe: cortar un comando de MPD a la mitad dejaría
/// la conexión desfasada. Los pedidos que llegan mientras tanto se pisan y solo se
/// atiende el último.
async fn serve<E, F>(
    mut service: LyricsService<E, F>,
    mut requests: watch::Receiver<Option<LyricsQuery>>,
    results: mpsc::UnboundedSender<LyricsResult>,
) where
    E: EmbeddedLyricsRepository + Send,
    F: LyricsFiles + Send,
{
    while requests.changed().await.is_ok() {
        let Some(query) = requests.borrow_and_update().clone() else {
            continue;
        };
        let outcome = service.find(&query).await;
        if results.send(LyricsResult { query, outcome }).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;

    use super::*;
    use crate::domain::{Song, TrackIdentity};
    use crate::repository::Error;

    /// Letras en memoria por fuente; `failing` hace fallar los tags.
    #[derive(Default)]
    struct Memory {
        cache: HashMap<String, String>,
        folder: HashMap<String, String>,
        tags: HashMap<String, String>,
        failing: bool,
    }

    impl EmbeddedLyricsRepository for Memory {
        async fn embedded_lyrics(&mut self, uri: &str) -> Result<Option<String>> {
            if self.failing {
                return Err(Error::Io(io::Error::other("se cortó")));
            }
            Ok(self.tags.get(uri).cloned())
        }
    }

    impl LyricsFiles for Memory {
        async fn cached(&mut self, track: &TrackIdentity) -> Result<Option<String>> {
            Ok(self.cache.get(track.title()).cloned())
        }

        async fn beside_song(&mut self, uri: &str) -> Result<Option<String>> {
            Ok(self.folder.get(uri).cloned())
        }
    }

    const URI: &str = "Rock/time.flac";

    fn query() -> LyricsQuery {
        LyricsQuery::for_song(&Song {
            file: URI.into(),
            artist: Some("Pink Floyd".into()),
            title: Some("Time".into()),
            ..Song::default()
        })
    }

    fn memory() -> Memory {
        Memory {
            cache: HashMap::from([("Time".into(), "[00:01]caché".into())]),
            folder: HashMap::from([(URI.into(), "carpeta".into())]),
            tags: HashMap::from([(URI.into(), "tags".into())]),
            failing: false,
        }
    }

    fn service(sources: &[LyricsSource]) -> LyricsService<Memory, Memory> {
        LyricsService::new(memory(), memory(), sources.to_vec())
    }

    fn found_source(outcome: LyricsOutcome) -> Option<LyricsSource> {
        match outcome {
            LyricsOutcome::Found(lyrics) => Some(lyrics.source),
            LyricsOutcome::NotFound { .. } => None,
        }
    }

    #[tokio::test]
    async fn the_first_source_with_lyrics_wins() {
        use LyricsSource::{Cache, SongFolder, Tags};
        let mut default_order = service(&[Cache, SongFolder, Tags]);
        assert_eq!(
            found_source(default_order.find(&query()).await),
            Some(Cache)
        );
        let mut tags_first = service(&[Tags, Cache]);
        assert_eq!(found_source(tags_first.find(&query()).await), Some(Tags));
        let mut only_folder = service(&[SongFolder]);
        assert_eq!(
            found_source(only_folder.find(&query()).await),
            Some(SongFolder)
        );
    }

    #[tokio::test]
    async fn the_cache_needs_artist_and_title() {
        let mut cache_only = service(&[LyricsSource::Cache]);
        let untitled = LyricsQuery {
            track: None,
            ..query()
        };
        assert_eq!(
            cache_only.find(&untitled).await,
            LyricsOutcome::NotFound { problem: None }
        );
    }

    #[tokio::test]
    async fn a_failing_source_does_not_stop_the_search() {
        let embedded = Memory {
            failing: true,
            ..memory()
        };
        let files = Memory {
            cache: HashMap::new(),
            ..memory()
        };
        let mut tags_first = LyricsService::new(
            embedded,
            files,
            vec![LyricsSource::Tags, LyricsSource::SongFolder],
        );
        assert_eq!(
            found_source(tags_first.find(&query()).await),
            Some(LyricsSource::SongFolder)
        );
        let mut only_tags = LyricsService::new(
            Memory {
                failing: true,
                ..memory()
            },
            memory(),
            vec![LyricsSource::Tags],
        );
        assert!(matches!(
            only_tags.find(&query()).await,
            LyricsOutcome::NotFound { problem: Some(_) }
        ));
    }

    #[tokio::test]
    async fn the_lookup_answers_the_latest_request() {
        let mut lookup = LyricsLookup::spawn(service(&[LyricsSource::Tags]));
        let other = LyricsQuery {
            uri: "otro.flac".into(),
            ..query()
        };
        lookup.request(other.clone());
        let first = lookup.next_result().await.unwrap();
        assert_eq!(first.query, other);
        assert_eq!(first.outcome, LyricsOutcome::NotFound { problem: None });
        lookup.request(query());
        let second = lookup.next_result().await.unwrap();
        assert_eq!(found_source(second.outcome), Some(LyricsSource::Tags));
    }
}
