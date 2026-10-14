//! Tests de punta a punta (Connection + MpdClient) contra un MPD falso con guion fijo.

use std::time::Duration;

use mpctui::domain::{
    Entry, PartitionName, Password, PlayState, PlaylistName, SampleFormat, SearchText, Status,
    Subsystem, Tag, TagFilter, Volume,
};
use mpctui::factory::{connect_following, connect_mpd};
use mpctui::repository::mpd::{Address, SharedPartition};
use mpctui::repository::{
    ChangesRepository, EmbeddedLyricsRepository, Error, LibraryRepository, OutputRepository,
    PartitionSelection, PlayerRepository, PlaylistRepository, QueueRepository,
};
use mpctui::service::{OutputService, UpdateTransition, update_transition};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const GREETING: &[u8] = b"OK MPD 0.23.5\n";
const LOOPBACK: &str = "127.0.0.1";

/// Un paso del guion: el comando que debe llegar y la respuesta a devolver.
type Step = (&'static str, &'static str);

/// Levanta un servidor que exige cada comando del guion en orden.
async fn fake_server(script: Vec<Step>) -> (Address, JoinHandle<()>) {
    let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();
        writer.write_all(GREETING).await.unwrap();
        for (expected_command, response) in script {
            let received = lines.next_line().await.unwrap();
            assert_eq!(received.as_deref(), Some(expected_command));
            writer.write_all(response.as_bytes()).await.unwrap();
        }
    });
    (Address::new(LOOPBACK, port), server)
}

#[tokio::test]
async fn reads_status() {
    let (address, server) = fake_server(vec![(
        "status",
        "volume: 80\nrepeat: 1\nrandom: 0\nsingle: 0\nconsume: 0\n\
         playlistlength: 2\nstate: pause\nsong: 1\nelapsed: 10.500\nduration: 200.000\n\
         bitrate: 245\naudio: 48000:f:2\nOK\n",
    )])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    let status = client.status().await.unwrap();
    assert_eq!(status.state, PlayState::Pause);
    assert_eq!(status.volume, Some(Volume::new(80).unwrap()));
    assert_eq!(status.elapsed, Some(Duration::from_millis(10_500)));
    assert_eq!(status.bitrate, Some(245));
    let audio = status.audio.unwrap();
    assert_eq!(audio.sample_rate, Some(48_000));
    assert_eq!(audio.format, SampleFormat::Float);
    assert_eq!(audio.channels, 2);
    server.await.unwrap();
}

#[tokio::test]
async fn authenticates_and_survives_ack() {
    let (address, server) = fake_server(vec![
        (r#"password "secreto""#, "OK\n"),
        (r#"play "99""#, "ACK [2@0] {play} Bad song index\n"),
        (r#"setvol "40""#, "OK\n"),
    ])
    .await;
    let password = Password::new("secreto").unwrap();
    let mut client = connect_mpd(&address, Some(&password)).await.unwrap();
    match client.play(Some(99)).await {
        Err(Error::Ack(ack)) => {
            assert_eq!(ack.code, 2);
            assert_eq!(ack.command, "play");
            assert_eq!(ack.message, "Bad song index");
        }
        other => panic!("se esperaba ACK, llegó {other:?}"),
    }
    client.set_volume(Volume::new(40).unwrap()).await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn reads_queue_and_current_song() {
    let (address, server) = fake_server(vec![
        (
            "playlistinfo",
            "file: a.mp3\nTitle: Uno\nduration: 100.0\nPos: 0\nId: 5\n\
             file: b.mp3\nArtist: X\nAlbumArtist: Y\nDate: 1997\nPos: 1\nId: 6\nOK\n",
        ),
        ("currentsong", "OK\n"),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    let queue = client.queue().await.unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].title.as_deref(), Some("Uno"));
    assert_eq!(queue[1].position, Some(1));
    assert_eq!(queue[1].album_owner(), Some("Y"));
    assert_eq!(queue[1].date.as_deref(), Some("1997"));
    assert_eq!(client.current_song().await.unwrap(), None);
    server.await.unwrap();
}

#[tokio::test]
async fn browses_lists_and_searches_library() {
    let (address, server) = fake_server(vec![
        (
            r#"lsinfo "Rock""#,
            "directory: Rock/Viejo\nfile: Rock/a.mp3\nTitle: A\nplaylist: Rock/l.m3u\nOK\n",
        ),
        (
            r#"list "Album" "(Artist == \"AC\\\"DC\")""#,
            "Album: Back in Black\nAlbum: Highway to Hell\nOK\n",
        ),
        (
            r#"search "(any contains \"black\")""#,
            "file: Rock/back.mp3\nTitle: Back in Black\nOK\n",
        ),
        (
            r#"findadd "((Artist == \"AC/DC\") AND (Album == \"Back in Black\"))""#,
            "OK\n",
        ),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();

    let entries = client.browse("Rock").await.unwrap();
    assert!(matches!(&entries[0], Entry::Directory { path } if path == "Rock/Viejo"));
    assert!(matches!(&entries[1], Entry::File(song) if song.file == "Rock/a.mp3"));
    assert!(matches!(&entries[2], Entry::Playlist { .. }));

    let albums = client
        .list_tag(Tag::Album, &[TagFilter::new(Tag::Artist, r#"AC"DC"#)])
        .await
        .unwrap();
    assert_eq!(albums, ["Back in Black", "Highway to Hell"]);

    let text = SearchText::new("black").unwrap();
    let found = client.search(Tag::Any, &text).await.unwrap();
    assert_eq!(found[0].display_title(), "Back in Black");

    let album = [
        TagFilter::new(Tag::Artist, "AC/DC"),
        TagFilter::new(Tag::Album, "Back in Black"),
    ];
    client.add_matching(&album).await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn manages_stored_playlists() {
    let (address, server) = fake_server(vec![
        (
            "listplaylists",
            "playlist: Rock\nLast-Modified: 2024-01-01T00:00:00Z\nOK\n",
        ),
        (r#"save "Mi lista""#, "OK\n"),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    assert_eq!(
        client.playlists().await.unwrap(),
        [PlaylistName::new("Rock").unwrap()]
    );
    client
        .save(&PlaylistName::new("Mi lista").unwrap())
        .await
        .unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn edits_the_queue_and_stored_playlists() {
    let (address, server) = fake_server(vec![
        (r#"move "3" "4""#, "OK\n"),
        (r#"playlistadd "Rock" "a/b.flac""#, "OK\n"),
        (r#"playlistdelete "Rock" "2""#, "OK\n"),
        (r#"playlistmove "Rock" "1" "0""#, "OK\n"),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    let rock = PlaylistName::new("Rock").unwrap();
    client.move_song(3, 4).await.unwrap();
    client.add_to_playlist(&rock, "a/b.flac").await.unwrap();
    client.delete_from_playlist(&rock, 2).await.unwrap();
    client.move_in_playlist(&rock, 1, 0).await.unwrap();
    server.await.unwrap();
}

/// Un comando en una partición: la lista llega línea por línea y MPD responde una sola
/// vez, al cerrarla.
fn in_partition(
    partition: &'static str,
    command: &'static str,
    response: &'static str,
) -> [Step; 4] {
    [
        ("command_list_begin", ""),
        (partition, ""),
        (command, ""),
        ("command_list_end", response),
    ]
}

#[tokio::test]
async fn lists_partitions_with_their_own_outputs() {
    let mut script = vec![(
        "listpartitions",
        "partition: default\npartition: cocina\nOK\n",
    )];
    script.extend(in_partition(
        r#"partition "default""#,
        "outputs",
        "outputid: 0\noutputname: pulse audio\nplugin: pulse\noutputenabled: 1\n\
         outputid: 1\noutputname: parlante\nplugin: dummy\noutputenabled: 0\nOK\n",
    ));
    script.extend(in_partition(
        r#"partition "cocina""#,
        "outputs",
        "outputid: 0\noutputname: pulse audio\nplugin: dummy\noutputenabled: 1\n\
         outputid: 1\noutputname: parlante\nplugin: alsa\noutputenabled: 0\n\
         attribute: dop=0\nOK\n",
    ));
    let (address, server) = fake_server(script).await;
    let repository = connect_mpd(&address, None).await.unwrap();
    let mut service = OutputService::new(repository, SharedPartition::default());
    let partitions = service.overview().await.unwrap();
    let owned: Vec<(&str, Vec<&str>)> = partitions
        .iter()
        .map(|partition| {
            let outputs = partition.outputs.iter().map(|output| output.name.as_str());
            (partition.name.as_str(), outputs.collect())
        })
        .collect();
    assert_eq!(
        owned,
        [
            ("default", vec!["pulse audio"]),
            ("cocina", vec!["parlante"])
        ]
    );
    assert_eq!(partitions[1].outputs[0].id, 1);
    server.await.unwrap();
}

#[tokio::test]
async fn manages_partitions_and_outputs() {
    let mut script = vec![(r#"newpartition "cocina""#, "OK\n")];
    script.extend(in_partition(
        r#"partition "cocina""#,
        r#"moveoutput "pulse audio""#,
        "OK\n",
    ));
    script.extend(in_partition(
        r#"partition "cocina""#,
        r#"enableoutput "1""#,
        "OK\n",
    ));
    script.extend(in_partition(
        r#"partition "default""#,
        r#"delpartition "cocina""#,
        "ACK [56@3] {delpartition} partition is busy\n",
    ));
    let (address, server) = fake_server(script).await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    let cocina = PartitionName::new("cocina").unwrap();
    client.create_partition(&cocina).await.unwrap();
    client.move_output("pulse audio", &cocina).await.unwrap();
    client.set_output_enabled(&cocina, 1, true).await.unwrap();
    match client.delete_partition(&cocina).await {
        Err(Error::Ack(ack)) => assert_eq!(ack.command, "delpartition"),
        other => panic!("se esperaba ACK, llegó {other:?}"),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn following_connections_enter_the_partition_in_use() {
    let (address, server) = fake_server(vec![
        ("playlistinfo", "OK\n"),
        (r#"partition "cocina""#, "OK\n"),
        ("playlistinfo", "file: a.flac\nPos: 0\nOK\n"),
        ("status", "partition: cocina\nstate: stop\nOK\n"),
        (r#"partition "default""#, "OK\n"),
        ("status", "state: play\nOK\n"),
        (
            r#"partition "vieja""#,
            "ACK [50@0] {partition} partition does not exist\n",
        ),
        ("status", "state: play\nOK\n"),
    ])
    .await;
    let partition = SharedPartition::default();
    let mut client = connect_following(&address, None, &partition).await.unwrap();
    assert!(client.queue().await.unwrap().is_empty());
    partition.select(PartitionName::new("cocina").unwrap());
    assert_eq!(client.queue().await.unwrap().len(), 1);
    assert_eq!(client.status().await.unwrap().state, PlayState::Stop);
    partition.select(PartitionName::default_partition());
    client.status().await.unwrap();
    partition.select(PartitionName::new("vieja").unwrap());
    assert!(matches!(client.status().await, Err(Error::Ack(ack)) if ack.code == 50));
    assert!(partition.active().is_default());
    assert_eq!(client.status().await.unwrap().state, PlayState::Play);
    server.await.unwrap();
}

#[tokio::test]
async fn seeks_relative_and_absolute() {
    let (address, server) = fake_server(vec![
        ("seekcur \"+5\"", "OK\n"),
        ("seekcur \"75.500\"", "OK\n"),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    client.seek_relative(5).await.unwrap();
    client.seek_to(Duration::from_millis(75_500)).await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn waits_for_changes() {
    let (address, server) =
        fake_server(vec![("idle", "changed: player\nchanged: mixer\nOK\n")]).await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    assert_eq!(
        client.wait_for_changes().await.unwrap(),
        [Subsystem::Player, Subsystem::Mixer]
    );
    server.await.unwrap();
}

#[tokio::test]
async fn database_update_starts_and_finishes() {
    let (address, server) = fake_server(vec![
        ("idle", "changed: update\nOK\n"),
        ("status", "state: play\nupdating_db: 3\nOK\n"),
        ("idle", "changed: update\nchanged: database\nOK\n"),
        ("status", "state: play\nOK\n"),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    let mut previous = Status::default();
    let mut transitions = Vec::new();
    for _ in 0..2 {
        let changed = client.wait_for_changes().await.unwrap();
        assert!(changed.contains(&Subsystem::Update));
        let status = client.status().await.unwrap();
        transitions.push(update_transition(&previous, &status));
        previous = status;
    }
    assert_eq!(
        transitions,
        [
            Some(UpdateTransition::Started),
            Some(UpdateTransition::Finished)
        ]
    );
    server.await.unwrap();
}

#[tokio::test]
async fn requests_a_database_update() {
    let (address, server) = fake_server(vec![("update", "updating_db: 4\nOK\n")]).await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    client.update_database().await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn pings_the_server() {
    let (address, server) = fake_server(vec![("ping", "OK\n")]).await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    client.ping().await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn reports_closed_connection() {
    let (address, server) = fake_server(vec![]).await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    server.await.unwrap();
    assert!(matches!(
        client.status().await,
        Err(Error::Closed | Error::Io(_))
    ));
}

#[tokio::test]
async fn reads_multiline_embedded_lyrics() {
    let (address, server) = fake_server(vec![
        (
            r#"readcomments "Rock/time.flac""#,
            "Artist: Pink Floyd\nlyrics: Ticking away\n\nVerso 2: otra línea\nTitle: Time\nOK\n",
        ),
        (r#"readcomments "Rock/otro.mp3""#, "TXXX: algo\nOK\n"),
    ])
    .await;
    let mut client = connect_mpd(&address, None).await.unwrap();
    let lyrics = client.embedded_lyrics("Rock/time.flac").await.unwrap();
    assert_eq!(
        lyrics.as_deref(),
        Some("Ticking away\n\nVerso 2: otra línea")
    );
    assert_eq!(client.embedded_lyrics("Rock/otro.mp3").await.unwrap(), None);
    server.await.unwrap();
}
