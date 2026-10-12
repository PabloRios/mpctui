//! Búsqueda de letras: prueba las fuentes en el orden configurado y gana la primera
//! que tenga letra. Lo que llega de LRCLIB se guarda en la caché (si `cache` es una de
//! las fuentes) y se recuerda durante la sesión, también lo que no se encontró, para no
//! repetir pedidos. La búsqueda corre en su propia tarea (`LyricsLookup`) para no
//! frenar la interfaz; si mientras tanto cambia el tema, se atiende solo el último
//! pedido.

use std::collections::HashMap;

use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use super::lrc::parse_lrc;
use crate::domain::{FetchedLyrics, Lyrics, LyricsQuery, LyricsSource, TrackIdentity};
use crate::repository::{EmbeddedLyricsRepository, Error, LyricsFiles, LyricsProvider, Result};

/// Resultado de una búsqueda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricsOutcome {
    Found(Lyrics),
    /// Una fuente dice que el tema no tiene letra.
    Instrumental,
    /// Ninguna fuente tenía letra. `problem` es el último error, si alguna falló.
    NotFound {
        problem: Option<Error>,
    },
}

pub struct LyricsService<E, F, P> {
    embedded: E,
    files: F,
    provider: P,
    /// En orden de prioridad, sin repetidos (lo valida la configuración).
    sources: Vec<LyricsSource>,
    /// Lo que respondió LRCLIB en esta sesión, por tema.
    remote_answers: HashMap<TrackIdentity, FetchedLyrics>,
}

impl<E, F, P> LyricsService<E, F, P>
where
    E: EmbeddedLyricsRepository + Send,
    F: LyricsFiles + Send,
    P: LyricsProvider + Send,
{
    pub fn new(embedded: E, files: F, provider: P, sources: Vec<LyricsSource>) -> Self {
        Self {
            embedded,
            files,
            provider,
            sources,
            remote_answers: HashMap::new(),
        }
    }

    /// Una fuente que falla no corta la búsqueda: se registra y se sigue con la próxima.
    pub async fn find(&mut self, query: &LyricsQuery) -> LyricsOutcome {
        let mut problem = None;
        for source in self.sources.clone() {
            match self.read(source, query).await {
                Ok(FetchedLyrics::Text(text)) => {
                    let lyrics = parse_lrc(&text, source);
                    if lyrics.lines.is_empty() {
                        continue;
                    }
                    if source == LyricsSource::Lrclib {
                        self.save_to_cache(query, &text, lyrics.is_synced()).await;
                    }
                    return LyricsOutcome::Found(lyrics);
                }
                Ok(FetchedLyrics::Instrumental) => return LyricsOutcome::Instrumental,
                Ok(FetchedLyrics::Missing) => {}
                Err(error) => {
                    tracing::warn!("no se pudo leer la letra ({source:?}): {error}");
                    problem = Some(error);
                }
            }
        }
        LyricsOutcome::NotFound { problem }
    }

    /// La caché y LRCLIB necesitan artista y título.
    async fn read(&mut self, source: LyricsSource, query: &LyricsQuery) -> Result<FetchedLyrics> {
        let local = match (source, &query.track) {
            (LyricsSource::Cache, Some(track)) => self.files.cached(track).await?,
            (LyricsSource::SongFolder, _) => self.files.beside_song(&query.uri).await?,
            (LyricsSource::Tags, _) => self.embedded.embedded_lyrics(&query.uri).await?,
            (LyricsSource::Lrclib, Some(track)) => return self.fetch_remote(track).await,
            (LyricsSource::Cache | LyricsSource::Lrclib, None) => None,
        };
        Ok(local.map_or(FetchedLyrics::Missing, FetchedLyrics::Text))
    }

    /// Cada tema se pide una sola vez por sesión; un error no se recuerda.
    async fn fetch_remote(&mut self, track: &TrackIdentity) -> Result<FetchedLyrics> {
        if let Some(answer) = self.remote_answers.get(track) {
            return Ok(answer.clone());
        }
        let answer = self.provider.fetch(track).await?;
        self.remote_answers.insert(track.clone(), answer.clone());
        Ok(answer)
    }

    /// Si no se puede guardar, la letra igual se muestra.
    async fn save_to_cache(&mut self, query: &LyricsQuery, text: &str, synced: bool) {
        let Some(track) = &query.track else {
            return;
        };
        if !self.sources.contains(&LyricsSource::Cache) {
            return;
        }
        if let Err(error) = self.files.save(track, text, synced).await {
            tracing::warn!("no se pudo guardar la letra en la caché: {error}");
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
    pub fn spawn<E, F, P>(service: LyricsService<E, F, P>) -> Self
    where
        E: EmbeddedLyricsRepository + Send + 'static,
        F: LyricsFiles + Send + 'static,
        P: LyricsProvider + Send + 'static,
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
async fn serve<E, F, P>(
    mut service: LyricsService<E, F, P>,
    mut requests: watch::Receiver<Option<LyricsQuery>>,
    results: mpsc::UnboundedSender<LyricsResult>,
) where
    E: EmbeddedLyricsRepository + Send,
    F: LyricsFiles + Send,
    P: LyricsProvider + Send,
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
    use std::io;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::domain::Song;
    use crate::repository::HttpFailure;

    /// LRCLIB no responde.
    const UNREACHABLE: Error = Error::Http {
        service: "LRCLIB",
        failure: HttpFailure::Connect,
    };

    /// Lo que pasó durante la búsqueda: pedidos a LRCLIB y archivos guardados.
    #[derive(Debug, Default)]
    struct Journal {
        fetches: usize,
        saved: Vec<(String, bool)>,
    }

    /// Letras en memoria por fuente; `failing` hace fallar los tags.
    #[derive(Default)]
    struct Memory {
        cache: HashMap<String, String>,
        folder: HashMap<String, String>,
        tags: HashMap<String, String>,
        remote: Option<FetchedLyrics>,
        failing: bool,
        journal: Arc<Mutex<Journal>>,
    }

    impl EmbeddedLyricsRepository for Memory {
        async fn embedded_lyrics(&mut self, uri: &str) -> Result<Option<String>> {
            if self.failing {
                return Err(io::Error::other("se cortó").into());
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

        async fn save(&mut self, _track: &TrackIdentity, text: &str, synced: bool) -> Result<()> {
            self.journal
                .lock()
                .unwrap()
                .saved
                .push((text.to_owned(), synced));
            Ok(())
        }
    }

    impl LyricsProvider for Memory {
        async fn fetch(&mut self, _track: &TrackIdentity) -> Result<FetchedLyrics> {
            self.journal.lock().unwrap().fetches += 1;
            self.remote.clone().ok_or(UNREACHABLE)
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
            remote: Some(FetchedLyrics::Text("[00:02]remota".into())),
            ..Memory::default()
        }
    }

    /// Sin nada local: solo responde LRCLIB con `remote`.
    fn remote_only(remote: Option<FetchedLyrics>, journal: &Arc<Mutex<Journal>>) -> Memory {
        Memory {
            remote,
            journal: Arc::clone(journal),
            ..Memory::default()
        }
    }

    type TestService = LyricsService<Memory, Memory, Memory>;

    fn service(sources: &[LyricsSource]) -> TestService {
        LyricsService::new(memory(), memory(), memory(), sources.to_vec())
    }

    fn with_files_and_provider(
        files: Memory,
        provider: Memory,
        sources: &[LyricsSource],
    ) -> TestService {
        LyricsService::new(Memory::default(), files, provider, sources.to_vec())
    }

    fn found_source(outcome: LyricsOutcome) -> Option<LyricsSource> {
        match outcome {
            LyricsOutcome::Found(lyrics) => Some(lyrics.source),
            LyricsOutcome::Instrumental | LyricsOutcome::NotFound { .. } => None,
        }
    }

    #[tokio::test]
    async fn the_first_source_with_lyrics_wins() {
        use LyricsSource::{Cache, Lrclib, SongFolder, Tags};
        let mut default_order = service(&[Cache, SongFolder, Tags, Lrclib]);
        assert_eq!(
            found_source(default_order.find(&query()).await),
            Some(Cache)
        );
        let mut tags_first = service(&[Tags, Cache]);
        assert_eq!(found_source(tags_first.find(&query()).await), Some(Tags));
        let mut remote_first = service(&[Lrclib, Cache]);
        assert_eq!(
            found_source(remote_first.find(&query()).await),
            Some(Lrclib)
        );
        let mut only_folder = service(&[SongFolder]);
        assert_eq!(
            found_source(only_folder.find(&query()).await),
            Some(SongFolder)
        );
    }

    #[tokio::test]
    async fn the_cache_and_lrclib_need_artist_and_title() {
        let mut remote = service(&[LyricsSource::Cache, LyricsSource::Lrclib]);
        let untitled = LyricsQuery {
            track: None,
            ..query()
        };
        assert_eq!(
            remote.find(&untitled).await,
            LyricsOutcome::NotFound { problem: None }
        );
    }

    #[tokio::test]
    async fn a_failing_source_does_not_stop_the_search() {
        let embedded = Memory {
            failing: true,
            ..memory()
        };
        let mut tags_first = LyricsService::new(
            embedded,
            memory(),
            memory(),
            vec![LyricsSource::Tags, LyricsSource::SongFolder],
        );
        assert_eq!(
            found_source(tags_first.find(&query()).await),
            Some(LyricsSource::SongFolder)
        );
        let journal = Arc::default();
        let mut offline = with_files_and_provider(
            Memory::default(),
            remote_only(None, &journal),
            &[LyricsSource::Lrclib],
        );
        assert_eq!(
            offline.find(&query()).await,
            LyricsOutcome::NotFound {
                problem: Some(UNREACHABLE)
            }
        );
    }

    #[tokio::test]
    async fn downloaded_lyrics_go_to_the_cache_only_if_it_is_a_source() {
        let journal: Arc<Mutex<Journal>> = Arc::default();
        let files = Memory {
            journal: Arc::clone(&journal),
            ..Memory::default()
        };
        let provider = remote_only(Some(FetchedLyrics::Text("texto plano".into())), &journal);
        let mut cached = with_files_and_provider(
            files,
            provider,
            &[LyricsSource::Cache, LyricsSource::Lrclib],
        );
        assert_eq!(
            found_source(cached.find(&query()).await),
            Some(LyricsSource::Lrclib)
        );
        assert_eq!(
            journal.lock().unwrap().saved,
            [("texto plano".to_owned(), false)]
        );

        let journal: Arc<Mutex<Journal>> = Arc::default();
        let files = Memory {
            journal: Arc::clone(&journal),
            ..Memory::default()
        };
        let provider = remote_only(Some(FetchedLyrics::Text("[00:01]x".into())), &journal);
        let mut uncached = with_files_and_provider(files, provider, &[LyricsSource::Lrclib]);
        assert_eq!(
            found_source(uncached.find(&query()).await),
            Some(LyricsSource::Lrclib)
        );
        assert!(journal.lock().unwrap().saved.is_empty());
    }

    #[tokio::test]
    async fn lrclib_is_asked_once_per_song() {
        let journal: Arc<Mutex<Journal>> = Arc::default();
        let provider = remote_only(Some(FetchedLyrics::Missing), &journal);
        let mut remote =
            with_files_and_provider(Memory::default(), provider, &[LyricsSource::Lrclib]);
        for _ in 0..3 {
            assert_eq!(
                remote.find(&query()).await,
                LyricsOutcome::NotFound { problem: None }
            );
        }
        assert_eq!(journal.lock().unwrap().fetches, 1);
    }

    #[tokio::test]
    async fn instrumentals_stop_the_search() {
        let journal = Arc::default();
        let provider = remote_only(Some(FetchedLyrics::Instrumental), &journal);
        let mut remote = LyricsService::new(
            memory(),
            Memory::default(),
            provider,
            vec![LyricsSource::Lrclib, LyricsSource::Tags],
        );
        assert_eq!(remote.find(&query()).await, LyricsOutcome::Instrumental);
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
