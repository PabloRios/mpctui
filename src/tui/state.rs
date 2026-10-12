//! Estado de la interfaz: qué se muestra y qué eligió el usuario.

use std::time::{Duration, Instant};

use ratatui::widgets::TableState;

use super::action::Navigation;
use super::list::{SelectableList, SelectionPolicy};
use super::prompt::Prompt;
use crate::domain::{
    Entry, PATH_SEPARATOR, PlaylistName, SearchText, Song, Tag, Timeline, timeline,
};
use crate::service::{LIBRARY_ROOT, PlaybackClock, PlayerSnapshot, SpectrumState};

/// Cuánto reemplaza un aviso a los atajos de la línea inferior.
pub const MESSAGE_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewKind {
    #[default]
    Queue,
    Library,
    Browse,
    Search,
    Playlists,
    Spectrum,
}

impl ViewKind {
    pub const ALL: [ViewKind; 6] = [
        ViewKind::Queue,
        ViewKind::Library,
        ViewKind::Browse,
        ViewKind::Search,
        ViewKind::Playlists,
        ViewKind::Spectrum,
    ];

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|view| *view == self)
            .unwrap_or_default()
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryColumn {
    #[default]
    Artists,
    Albums,
    Songs,
}

impl LibraryColumn {
    pub fn next(self) -> Self {
        match self {
            LibraryColumn::Artists => LibraryColumn::Albums,
            LibraryColumn::Albums | LibraryColumn::Songs => LibraryColumn::Songs,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            LibraryColumn::Artists | LibraryColumn::Albums => LibraryColumn::Artists,
            LibraryColumn::Songs => LibraryColumn::Albums,
        }
    }
}

/// Lista de temas que se muestra agrupada por álbum. La línea de tiempo se recalcula
/// solo cuando cambian los temas, nunca al dibujar.
#[derive(Debug, Default)]
pub struct GroupedSongs {
    songs: SelectableList<Song>,
    timeline: Timeline,
}

impl GroupedSongs {
    pub fn set_items(&mut self, songs: Vec<Song>, policy: SelectionPolicy) {
        self.timeline = timeline(&songs);
        self.songs.set_items(songs, policy);
    }

    pub fn clear(&mut self) {
        self.set_items(Vec::new(), SelectionPolicy::Reset);
    }

    pub fn items(&self) -> &[Song] {
        self.songs.items()
    }

    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    pub fn selected(&self) -> Option<&Song> {
        self.songs.selected()
    }

    pub fn navigate(&mut self, navigation: Navigation) -> bool {
        self.songs.navigate(navigation)
    }

    pub fn select_where(&mut self, predicate: impl Fn(&Song) -> bool) {
        self.songs.select_where(predicate);
    }

    pub fn request_centering(&mut self) {
        self.songs.request_centering();
    }

    /// Todo lo que hace falta para dibujar, sin copiar los temas.
    pub fn render_parts(&mut self) -> (&[Song], &Timeline, &mut TableState, bool) {
        let centering = self.songs.take_centering_request();
        let (songs, table) = self.songs.parts_mut();
        (songs, &self.timeline, table, centering)
    }
}

/// Biblioteca en tres columnas: Artista → Álbum → Tema.
#[derive(Debug, Default)]
pub struct LibraryView {
    pub artists: SelectableList<String>,
    pub albums: SelectableList<String>,
    pub songs: SelectableList<Song>,
    pub focus: LibraryColumn,
}

impl LibraryView {
    pub fn selected_artist(&self) -> Option<String> {
        self.artists.selected().cloned()
    }

    pub fn selected_album(&self) -> Option<(String, String)> {
        Some((self.selected_artist()?, self.albums.selected()?.clone()))
    }
}

/// Navegación por directorios.
#[derive(Debug)]
pub struct BrowseView {
    pub path: String,
    pub entries: SelectableList<Entry>,
}

impl Default for BrowseView {
    fn default() -> Self {
        Self {
            path: LIBRARY_ROOT.to_owned(),
            entries: SelectableList::default(),
        }
    }
}

impl BrowseView {
    /// Directorio padre, o `None` si ya está en la raíz.
    pub fn parent_path(&self) -> Option<String> {
        if self.path == LIBRARY_ROOT {
            return None;
        }
        let parent = match self.path.rsplit_once(PATH_SEPARATOR) {
            Some((parent, _)) => parent,
            None => LIBRARY_ROOT,
        };
        Some(parent.to_owned())
    }
}

/// Campos de búsqueda en el orden en que rotan con Tab.
pub const SEARCH_FIELDS: [Tag; 4] = [Tag::Any, Tag::Artist, Tag::Album, Tag::Title];

#[derive(Debug)]
pub struct SearchView {
    pub field: Tag,
    pub text: Option<SearchText>,
    pub results: GroupedSongs,
}

impl Default for SearchView {
    fn default() -> Self {
        Self {
            field: SEARCH_FIELDS[0],
            text: None,
            results: GroupedSongs::default(),
        }
    }
}

impl SearchView {
    pub fn cycle_field(&mut self) {
        let index = SEARCH_FIELDS
            .iter()
            .position(|field| *field == self.field)
            .unwrap_or_default();
        self.field = SEARCH_FIELDS[(index + 1) % SEARCH_FIELDS.len()];
    }
}

#[derive(Debug, Default)]
pub struct PlaylistsView {
    pub names: SelectableList<PlaylistName>,
    /// Temas de la playlist seleccionada.
    pub preview: GroupedSongs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeLevel {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub level: NoticeLevel,
    pub text: String,
    pub shown_at: Instant,
}

impl Notice {
    /// Un aviso se muestra durante `MESSAGE_DURATION`.
    pub fn is_visible_at(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.shown_at) < MESSAGE_DURATION
    }
}

/// Visualizador de espectro: lo último que publicó y qué fuente lee.
#[derive(Debug, Default)]
pub struct SpectrumView {
    pub state: SpectrumState,
    /// Ruta del FIFO (`/tmp/mpd.fifo`).
    pub source: String,
    /// Formato que se espera en el FIFO (`44100:16:2`).
    pub format: String,
}

/// Estado de la conexión con el servidor, según el vigilante de cambios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Connected,
    /// Sin conexión: el intento `attempt` será en `retry_at`, tras esperar `delay`.
    Waiting {
        attempt: u32,
        retry_at: Instant,
        delay: Duration,
    },
    /// Intentando reconectar.
    Connecting { attempt: u32 },
}

/// Todo lo que la vista necesita para dibujarse.
#[derive(Debug, Default)]
pub struct AppState {
    pub running: bool,
    pub view: ViewKind,
    pub help_visible: bool,
    pub prompt: Option<Prompt>,
    pub notice: Option<Notice>,
    pub connection: ConnectionState,
    /// Cuándo se vio por primera vez la actualización de la base en curso.
    pub database_update: Option<Instant>,
    /// Servidor al que se conecta, para el encabezado.
    pub server: String,
    /// Ticks transcurridos; anima los spinners.
    pub ticks: usize,
    pub player: PlayerSnapshot,
    pub clock: PlaybackClock,
    pub queue: GroupedSongs,
    pub library: LibraryView,
    pub browse: BrowseView,
    pub search: SearchView,
    pub playlists: PlaylistsView,
    pub spectrum: SpectrumView,
}

impl AppState {
    pub fn new(server: String) -> Self {
        Self {
            running: true,
            server,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_cycle_in_both_directions() {
        assert_eq!(ViewKind::Queue.next(), ViewKind::Library);
        assert_eq!(ViewKind::Playlists.next(), ViewKind::Spectrum);
        assert_eq!(ViewKind::Spectrum.next(), ViewKind::Queue);
        assert_eq!(ViewKind::Queue.previous(), ViewKind::Spectrum);
    }

    #[test]
    fn library_focus_stops_at_edges() {
        assert_eq!(LibraryColumn::Songs.next(), LibraryColumn::Songs);
        assert_eq!(LibraryColumn::Artists.previous(), LibraryColumn::Artists);
        assert_eq!(LibraryColumn::Artists.next(), LibraryColumn::Albums);
    }

    #[test]
    fn parent_path_walks_up_to_root() {
        let mut browse = BrowseView::default();
        assert_eq!(browse.parent_path(), None);
        browse.path = "Rock/Viejo".into();
        assert_eq!(browse.parent_path().as_deref(), Some("Rock"));
        browse.path = "Rock".into();
        assert_eq!(browse.parent_path().as_deref(), Some(""));
    }

    #[test]
    fn grouped_songs_recompute_the_timeline() {
        let mut songs = GroupedSongs::default();
        let song = Song {
            file: "a.mp3".into(),
            album: Some("A".into()),
            ..Song::default()
        };
        songs.set_items(vec![song.clone(), song], SelectionPolicy::Reset);
        assert_eq!(songs.timeline().runs.len(), 1);
        songs.clear();
        assert!(songs.timeline().runs.is_empty());
    }

    #[test]
    fn notices_expire() {
        let shown_at = Instant::now();
        let notice = Notice {
            level: NoticeLevel::Info,
            text: "hola".into(),
            shown_at,
        };
        assert!(notice.is_visible_at(shown_at));
        assert!(!notice.is_visible_at(shown_at + MESSAGE_DURATION));
    }

    #[test]
    fn search_field_cycles() {
        let mut search = SearchView::default();
        assert_eq!(search.field, Tag::Any);
        for _ in 0..SEARCH_FIELDS.len() {
            search.cycle_field();
        }
        assert_eq!(search.field, Tag::Any);
    }
}
