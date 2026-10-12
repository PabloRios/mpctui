//! Implementación de los repositorios sobre el protocolo de MPD.

use super::parse::{
    parse_entries, parse_playlist_names, parse_songs, parse_status, parse_subsystems, values_of,
};
use super::protocol::{Command, FilterOperator, Pair, filter_expression};
use super::transport::Transport;
use super::vocabulary::{Verb, flag, single_mode_argument, tag_name};
use crate::domain::{
    Entry, PlaylistName, SearchText, SingleMode, Song, Status, Subsystem, Tag, TagFilter, Volume,
};
use crate::repository::error::Result;
use crate::repository::traits::{
    ChangesRepository, LibraryRepository, OptionsRepository, PlayerRepository, PlaylistRepository,
    QueueRepository,
};

pub struct MpdClient<T> {
    transport: T,
}

impl<T: Transport + Send> MpdClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Ejecuta un comando y devuelve sus pares.
    async fn fetch(&mut self, command: Command) -> Result<Vec<Pair>> {
        self.transport.execute(&command).await
    }

    /// Ejecuta un comando cuya respuesta no tiene datos.
    async fn run(&mut self, command: Command) -> Result<()> {
        self.fetch(command).await.map(drop)
    }
}

impl<T: Transport + Send> PlayerRepository for MpdClient<T> {
    async fn status(&mut self) -> Result<Status> {
        parse_status(&self.fetch(Command::new(Verb::Status)).await?)
    }

    async fn current_song(&mut self) -> Result<Option<Song>> {
        let songs = parse_songs(&self.fetch(Command::new(Verb::CurrentSong)).await?)?;
        Ok(songs.into_iter().next())
    }

    async fn play(&mut self, position: Option<u32>) -> Result<()> {
        self.run(Command::new(Verb::Play).optional_arg(position))
            .await
    }

    async fn set_pause(&mut self, paused: bool) -> Result<()> {
        self.run(Command::new(Verb::Pause).arg(flag(paused))).await
    }

    async fn stop(&mut self) -> Result<()> {
        self.run(Command::new(Verb::Stop)).await
    }

    async fn next(&mut self) -> Result<()> {
        self.run(Command::new(Verb::Next)).await
    }

    async fn previous(&mut self) -> Result<()> {
        self.run(Command::new(Verb::Previous)).await
    }

    async fn set_volume(&mut self, volume: Volume) -> Result<()> {
        self.run(Command::new(Verb::SetVolume).arg(volume.value()))
            .await
    }

    async fn seek_relative(&mut self, offset_seconds: i32) -> Result<()> {
        let offset = format!("{offset_seconds:+}");
        self.run(Command::new(Verb::SeekCurrent).arg(offset)).await
    }
}

impl<T: Transport + Send> OptionsRepository for MpdClient<T> {
    async fn set_random(&mut self, enabled: bool) -> Result<()> {
        self.run(Command::new(Verb::Random).arg(flag(enabled)))
            .await
    }

    async fn set_repeat(&mut self, enabled: bool) -> Result<()> {
        self.run(Command::new(Verb::Repeat).arg(flag(enabled)))
            .await
    }

    async fn set_single(&mut self, mode: SingleMode) -> Result<()> {
        self.run(Command::new(Verb::Single).arg(single_mode_argument(mode)))
            .await
    }

    async fn set_consume(&mut self, enabled: bool) -> Result<()> {
        self.run(Command::new(Verb::Consume).arg(flag(enabled)))
            .await
    }
}

impl<T: Transport + Send> QueueRepository for MpdClient<T> {
    async fn queue(&mut self) -> Result<Vec<Song>> {
        parse_songs(&self.fetch(Command::new(Verb::QueueInfo)).await?)
    }

    async fn add(&mut self, uri: &str) -> Result<()> {
        self.run(Command::new(Verb::Add).arg(uri)).await
    }

    async fn add_matching(&mut self, filters: &[TagFilter]) -> Result<()> {
        let expression = filter_expression(filters, FilterOperator::Equals);
        self.run(Command::new(Verb::FindAdd).optional_arg(expression))
            .await
    }

    async fn delete(&mut self, position: u32) -> Result<()> {
        self.run(Command::new(Verb::Delete).arg(position)).await
    }

    async fn clear(&mut self) -> Result<()> {
        self.run(Command::new(Verb::Clear)).await
    }
}

impl<T: Transport + Send> LibraryRepository for MpdClient<T> {
    async fn browse(&mut self, path: &str) -> Result<Vec<Entry>> {
        let directory = Some(path).filter(|path| !path.is_empty());
        parse_entries(
            &self
                .fetch(Command::new(Verb::ListInfo).optional_arg(directory))
                .await?,
        )
    }

    async fn list_tag(&mut self, tag: Tag, filters: &[TagFilter]) -> Result<Vec<String>> {
        let expression = filter_expression(filters, FilterOperator::Equals);
        let command = Command::new(Verb::List)
            .arg(tag_name(tag))
            .optional_arg(expression);
        Ok(values_of(&self.fetch(command).await?, tag_name(tag)).collect())
    }

    async fn find(&mut self, filters: &[TagFilter]) -> Result<Vec<Song>> {
        let expression = filter_expression(filters, FilterOperator::Equals);
        parse_songs(
            &self
                .fetch(Command::new(Verb::Find).optional_arg(expression))
                .await?,
        )
    }

    async fn search(&mut self, field: Tag, text: &SearchText) -> Result<Vec<Song>> {
        let filters = [TagFilter::new(field, text.as_str())];
        let expression = filter_expression(&filters, FilterOperator::Contains);
        parse_songs(
            &self
                .fetch(Command::new(Verb::Search).optional_arg(expression))
                .await?,
        )
    }
}

impl<T: Transport + Send> PlaylistRepository for MpdClient<T> {
    async fn playlists(&mut self) -> Result<Vec<PlaylistName>> {
        parse_playlist_names(&self.fetch(Command::new(Verb::ListPlaylists)).await?)
    }

    async fn playlist_songs(&mut self, name: &PlaylistName) -> Result<Vec<Song>> {
        parse_songs(
            &self
                .fetch(Command::new(Verb::ListPlaylistInfo).arg(name))
                .await?,
        )
    }

    async fn load(&mut self, name: &PlaylistName) -> Result<()> {
        self.run(Command::new(Verb::Load).arg(name)).await
    }

    async fn save(&mut self, name: &PlaylistName) -> Result<()> {
        self.run(Command::new(Verb::Save).arg(name)).await
    }

    async fn remove(&mut self, name: &PlaylistName) -> Result<()> {
        self.run(Command::new(Verb::Remove).arg(name)).await
    }
}

impl<T: Transport + Send> ChangesRepository for MpdClient<T> {
    async fn wait_for_changes(&mut self) -> Result<Vec<Subsystem>> {
        Ok(parse_subsystems(
            &self.fetch(Command::new(Verb::Idle)).await?,
        ))
    }

    async fn ping(&mut self) -> Result<()> {
        self.run(Command::new(Verb::Ping)).await
    }
}
