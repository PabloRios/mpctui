//! Implementación de los repositorios sobre el protocolo de MPD.

use std::time::Duration;

use super::parse::{
    parse_entries, parse_outputs, parse_partition_names, parse_playlist_names, parse_songs,
    parse_status, parse_subsystems, values_of,
};
use super::protocol::{Command, FilterOperator, Pair, filter_expression};
use super::transport::Transport;
use super::vocabulary::{SEEK_DECIMALS, Verb, flag, keys, single_mode_argument, tag_name};
use crate::domain::{
    Entry, Output, PartitionName, PlaylistName, SearchText, SingleMode, Song, Status, Subsystem,
    Tag, TagFilter, Volume,
};
use crate::repository::error::Result;
use crate::repository::traits::{
    ChangesRepository, EmbeddedLyricsRepository, LibraryRepository, OptionsRepository,
    OutputRepository, PlayerRepository, PlaylistRepository, QueueRepository,
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

    /// Ejecuta `command` en `partition`: los dos van en una lista, así un reintento
    /// tras reconectar no lo ejecuta en otra partición.
    async fn fetch_in(&mut self, partition: &PartitionName, command: Command) -> Result<Vec<Pair>> {
        let switch = Command::new(Verb::Partition).arg(partition);
        self.fetch(Command::list([switch, command])).await
    }

    async fn run_in(&mut self, partition: &PartitionName, command: Command) -> Result<()> {
        self.fetch_in(partition, command).await.map(drop)
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

    async fn seek_to(&mut self, position: Duration) -> Result<()> {
        let seconds = format!("{:.SEEK_DECIMALS$}", position.as_secs_f64());
        self.run(Command::new(Verb::SeekCurrent).arg(seconds)).await
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

    async fn move_song(&mut self, from: u32, to: u32) -> Result<()> {
        self.run(Command::new(Verb::Move).arg(from).arg(to)).await
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

    async fn update_database(&mut self) -> Result<()> {
        self.run(Command::new(Verb::Update)).await
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

    async fn add_to_playlist(&mut self, name: &PlaylistName, uri: &str) -> Result<()> {
        self.run(Command::new(Verb::PlaylistAdd).arg(name).arg(uri))
            .await
    }

    async fn delete_from_playlist(&mut self, name: &PlaylistName, position: u32) -> Result<()> {
        self.run(Command::new(Verb::PlaylistDelete).arg(name).arg(position))
            .await
    }

    async fn move_in_playlist(&mut self, name: &PlaylistName, from: u32, to: u32) -> Result<()> {
        self.run(Command::new(Verb::PlaylistMove).arg(name).arg(from).arg(to))
            .await
    }
}

impl<T: Transport + Send> OutputRepository for MpdClient<T> {
    async fn partitions(&mut self) -> Result<Vec<PartitionName>> {
        parse_partition_names(&self.fetch(Command::new(Verb::ListPartitions)).await?)
    }

    async fn outputs(&mut self, partition: &PartitionName) -> Result<Vec<Output>> {
        parse_outputs(
            &self
                .fetch_in(partition, Command::new(Verb::Outputs))
                .await?,
        )
    }

    async fn create_partition(&mut self, name: &PartitionName) -> Result<()> {
        self.run(Command::new(Verb::NewPartition).arg(name)).await
    }

    /// Desde `default`: la propia conexión no puede estar en la que se borra.
    async fn delete_partition(&mut self, name: &PartitionName) -> Result<()> {
        let command = Command::new(Verb::DeletePartition).arg(name);
        self.run_in(&PartitionName::default_partition(), command)
            .await
    }

    async fn move_output(&mut self, output: &str, to: &PartitionName) -> Result<()> {
        self.run_in(to, Command::new(Verb::MoveOutput).arg(output))
            .await
    }

    async fn set_output_enabled(
        &mut self,
        partition: &PartitionName,
        output_id: u32,
        enabled: bool,
    ) -> Result<()> {
        let verb = if enabled {
            Verb::EnableOutput
        } else {
            Verb::DisableOutput
        };
        self.run_in(partition, Command::new(verb).arg(output_id))
            .await
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

impl<T: Transport + Send> EmbeddedLyricsRepository for MpdClient<T> {
    async fn embedded_lyrics(&mut self, uri: &str) -> Result<Option<String>> {
        let comments = self
            .fetch(Command::new(Verb::ReadComments).arg(uri))
            .await?;
        Ok(comments
            .into_iter()
            .find(|(key, _)| {
                keys::LYRICS_COMMENTS
                    .iter()
                    .any(|lyrics| key.eq_ignore_ascii_case(lyrics))
            })
            .map(|(_, value)| value))
    }
}
