//! Estado de la interfaz: qué se muestra y qué eligió el usuario.

use std::time::{Duration, Instant};

use ratatui::widgets::TableState;

use super::action::Navigation;
use super::list::{SelectableList, SelectionPolicy};
use super::picker::Picker;
use super::prompt::Prompt;
use crate::domain::{
    Entry, Lyrics, LyricsQuery, Output, PATH_SEPARATOR, Partition, PartitionName, PlaylistName,
    STEREO_CHANNELS, SearchText, Song, SongDetails, Tag, Timeline, VisualRequest, VisualizerStyle,
    timeline,
};
use crate::repository::Error;
use crate::service::{LIBRARY_ROOT, PlaybackClock, PlayerSnapshot, VisualState};

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
    Visualizer,
    Lyrics,
    Outputs,
}

impl ViewKind {
    pub const ALL: [ViewKind; 8] = [
        ViewKind::Queue,
        ViewKind::Library,
        ViewKind::Browse,
        ViewKind::Search,
        ViewKind::Playlists,
        ViewKind::Visualizer,
        ViewKind::Lyrics,
        ViewKind::Outputs,
    ];
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

    pub fn selected_index(&self) -> Option<usize> {
        self.songs.selected_index()
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

/// Columna con foco en la vista Playlists: los nombres o los temas de la seleccionada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaylistsColumn {
    #[default]
    Names,
    Songs,
}

#[derive(Debug, Default)]
pub struct PlaylistsView {
    pub names: SelectableList<PlaylistName>,
    /// Temas de la playlist seleccionada.
    pub preview: GroupedSongs,
    pub focus: PlaylistsColumn,
}

/// Columna con foco en la vista Salidas: las particiones o las salidas de la elegida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputsColumn {
    #[default]
    Partitions,
    Outputs,
}

/// Particiones a la izquierda y salidas de la elegida a la derecha.
#[derive(Debug, Default)]
pub struct OutputsView {
    pub partitions: SelectableList<Partition>,
    pub outputs: SelectableList<Output>,
    pub focus: OutputsColumn,
}

impl OutputsView {
    pub fn selected_partition(&self) -> Option<&Partition> {
        self.partitions.selected()
    }

    /// Carga las particiones conservando la elegida por nombre aunque cambie de lugar;
    /// si ya no está, sus salidas vuelven al principio y el foco a las particiones.
    pub fn set_partitions(&mut self, partitions: Vec<Partition>) {
        let previous = self.selected_name();
        self.partitions.set_items(partitions, SelectionPolicy::Keep);
        if let Some(previous) = &previous {
            self.partitions
                .select_where(|partition| partition.name == *previous);
        }
        let policy = if self.selected_name() == previous {
            SelectionPolicy::Keep
        } else {
            SelectionPolicy::Reset
        };
        self.show_selected_outputs(policy);
    }

    /// Las salidas de la partición elegida pasan a la columna derecha.
    pub fn show_selected_outputs(&mut self, policy: SelectionPolicy) {
        let outputs = self
            .selected_partition()
            .map(|partition| partition.outputs.clone())
            .unwrap_or_default();
        if outputs.is_empty() {
            self.focus = OutputsColumn::Partitions;
        }
        self.outputs.set_items(outputs, policy);
    }

    fn selected_name(&self) -> Option<PartitionName> {
        self.selected_partition()
            .map(|partition| partition.name.clone())
    }
}

/// Ficha de un tema abierta. Se guarda el tema para sacar el códec de su extensión.
#[derive(Debug)]
pub struct SongInfoView {
    pub song: Song,
    pub details: SongDetails,
    /// Primera fila visible; la vista la limita al dibujar.
    pub scroll: usize,
}

impl SongInfoView {
    pub fn new(song: Song, details: SongDetails) -> Self {
        Self {
            song,
            details,
            scroll: 0,
        }
    }
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

/// Visualizador: lo último que publicó, la forma activa y qué fuente lee.
#[derive(Debug)]
pub struct VisualView {
    pub state: VisualState,
    /// Forma activa; se conserva al salir de la vista.
    pub style: VisualizerStyle,
    /// Lo que pidió el último dibujado según el área; la app se lo pasa al visualizador.
    pub request: Option<VisualRequest>,
    /// Ruta del FIFO (`/tmp/mpd.fifo`).
    pub source: String,
    /// Formato que se espera en el FIFO (`44100:16:2`).
    pub format: String,
    /// Canales del FIFO: con uno, las formas estéreo salen del ciclo.
    pub channels: u8,
}

impl Default for VisualView {
    fn default() -> Self {
        Self {
            state: VisualState::default(),
            style: VisualizerStyle::default(),
            request: None,
            source: String::new(),
            format: String::new(),
            channels: STEREO_CHANNELS,
        }
    }
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
    /// Selector abierto: de playlist (`P`) o de partición (`m`).
    pub picker: Option<Picker>,
    /// Ficha del tema (`i`).
    pub song_info: Option<SongInfoView>,
    pub notice: Option<Notice>,
    pub connection: ConnectionState,
    /// Cuándo se vio por primera vez la actualización de la base en curso.
    pub database_update: Option<Instant>,
    /// Servidor al que se conecta, para el encabezado.
    pub server: String,
    /// La partición que usa mpctui.
    pub partition: PartitionName,
    /// Ticks transcurridos; anima los spinners.
    pub ticks: usize,
    pub player: PlayerSnapshot,
    pub clock: PlaybackClock,
    pub queue: GroupedSongs,
    pub library: LibraryView,
    pub browse: BrowseView,
    pub search: SearchView,
    pub playlists: PlaylistsView,
    pub outputs: OutputsView,
    pub visual: VisualView,
    pub lyrics: LyricsView,
    /// Con las letras desactivadas, la vista no aparece ni entra en el ciclo de `Tab`.
    pub lyrics_enabled: bool,
}

impl AppState {
    pub fn new(server: String) -> Self {
        Self {
            running: true,
            server,
            ..Self::default()
        }
    }

    /// Las vistas que se muestran, en el orden de las pestañas.
    pub fn views(&self) -> Vec<ViewKind> {
        ViewKind::ALL
            .into_iter()
            .filter(|view| *view != ViewKind::Lyrics || self.lyrics_enabled)
            .collect()
    }

    pub fn next_view(&self) -> ViewKind {
        self.view_at_offset(1)
    }

    pub fn previous_view(&self) -> ViewKind {
        self.view_at_offset(-1)
    }

    /// La vista a `offset` pestañas de la activa, dando la vuelta.
    fn view_at_offset(&self, offset: isize) -> ViewKind {
        let views = self.views();
        let count = views.len() as isize;
        let current = views
            .iter()
            .position(|view| *view == self.view)
            .unwrap_or_default() as isize;
        views[(current + offset).rem_euclid(count) as usize]
    }
}

/// Qué muestra la vista Letra.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LyricsStatus {
    /// No suena ni hay tema elegido.
    #[default]
    NoSong,
    Searching,
    Found(Lyrics),
    /// El tema no tiene letra (según LRCLIB).
    Instrumental,
    /// `untitled`: al tema le faltan artista o título, así que no se buscó en la caché.
    NotFound {
        untitled: bool,
        problem: Option<Error>,
    },
}

impl LyricsStatus {
    /// Solo se reintenta una búsqueda en la que falló alguna fuente (red, MPD, disco).
    pub fn can_retry(&self) -> bool {
        matches!(
            self,
            LyricsStatus::NotFound {
                problem: Some(_),
                ..
            }
        )
    }
}

#[derive(Debug, Default)]
pub struct LyricsView {
    /// El último tema pedido; si el actual es otro, hay que buscar de nuevo.
    pub query: Option<LyricsQuery>,
    pub status: LyricsStatus,
    /// Primera línea visible de una letra sin sincronizar.
    pub scroll: usize,
    /// Línea elegida en una letra sincronizada; `None` sigue la canción.
    pub cursor: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_cycle_in_both_directions() {
        let mut state = AppState {
            lyrics_enabled: true,
            ..AppState::default()
        };
        assert_eq!(state.next_view(), ViewKind::Library);
        assert_eq!(state.previous_view(), ViewKind::Outputs);
        state.view = ViewKind::Lyrics;
        assert_eq!(state.next_view(), ViewKind::Outputs);
        state.view = ViewKind::Outputs;
        assert_eq!(state.next_view(), ViewKind::Queue);
    }

    #[test]
    fn disabled_lyrics_leave_the_cycle() {
        let mut state = AppState::default();
        assert_eq!(state.views().len(), ViewKind::ALL.len() - 1);
        assert_eq!(state.previous_view(), ViewKind::Outputs);
        state.view = ViewKind::Visualizer;
        assert_eq!(state.next_view(), ViewKind::Outputs);
    }

    fn partition(name: &str, outputs: &[&str]) -> Partition {
        Partition {
            name: PartitionName::new(name).unwrap(),
            outputs: outputs
                .iter()
                .zip(0..)
                .map(|(output, id)| Output {
                    id,
                    name: (*output).into(),
                    plugin: "alsa".into(),
                    enabled: true,
                })
                .collect(),
        }
    }

    fn output_names(view: &OutputsView) -> Vec<&str> {
        view.outputs
            .items()
            .iter()
            .map(|output| output.name.as_str())
            .collect()
    }

    #[test]
    fn reloading_partitions_keeps_the_chosen_one_by_name() {
        let mut view = OutputsView::default();
        view.set_partitions(vec![
            partition("default", &["a"]),
            partition("zona", &["b", "c"]),
        ]);
        view.partitions.navigate(Navigation::Next);
        view.show_selected_outputs(SelectionPolicy::Reset);
        view.outputs.navigate(Navigation::Next);
        view.focus = OutputsColumn::Outputs;
        view.set_partitions(vec![
            partition("default", &["a"]),
            partition("cocina", &[]),
            partition("zona", &["b", "c"]),
        ]);
        assert_eq!(view.selected_partition().unwrap().name.as_str(), "zona");
        assert_eq!(view.outputs.selected().unwrap().name, "c");
        assert_eq!(view.focus, OutputsColumn::Outputs);
    }

    #[test]
    fn a_partition_without_outputs_takes_the_focus_back() {
        let mut view = OutputsView {
            focus: OutputsColumn::Outputs,
            ..OutputsView::default()
        };
        view.set_partitions(vec![partition("cocina", &[])]);
        assert!(output_names(&view).is_empty());
        assert_eq!(view.focus, OutputsColumn::Partitions);
        view.set_partitions(vec![partition("default", &["a", "b"])]);
        assert_eq!(output_names(&view), ["a", "b"]);
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

    #[test]
    fn only_failed_searches_can_be_retried() {
        let failed = LyricsStatus::NotFound {
            untitled: false,
            problem: Some(Error::Closed),
        };
        assert!(failed.can_retry());
        let missing = LyricsStatus::NotFound {
            untitled: false,
            problem: None,
        };
        assert!(!missing.can_retry());
        assert!(!LyricsStatus::Searching.can_retry());
        assert!(!LyricsStatus::Instrumental.can_retry());
    }
}
