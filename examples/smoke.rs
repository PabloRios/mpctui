//! Prueba de humo de solo lectura contra un MPD real.
//! Uso: `cargo run --example smoke [host] [port]`. Sin argumentos, MPD se busca como en
//! la app: `config.toml`, `MPD_HOST`/`MPD_PORT` y, si no hay host, los sockets locales y
//! después `localhost`.

use mpctui::config;
use mpctui::domain::{SearchText, Tag, TagFilter};
use mpctui::factory::connect_mpd;
use mpctui::repository::mpd::{
    Address, AddressCandidates, ConnectionSettings, DEFAULT_PORT, first_reachable,
};
use mpctui::repository::{
    LibraryRepository, OutputRepository, PlayerRepository, PlaylistRepository, QueueRepository,
};

const SAMPLE_SEARCH: &str = "love";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (candidates, connection) = server_from_arguments()?;
    let address = first_reachable(&candidates, &connection).await?;
    let mut client = connect_mpd(&address, &connection).await?;
    println!("Conectado a {address}");

    let status = client.status().await?;
    println!(
        "Estado: {:?}, volumen {:?}, cola {} temas",
        status.state, status.volume, status.queue_length
    );
    if let Some(song) = client.current_song().await? {
        println!(
            "Sonando: {} - {} [{:?} / {:?}]",
            song.artist.as_deref().unwrap_or("?"),
            song.display_title(),
            status.elapsed,
            song.duration
        );
    }
    println!("Cola leída: {} temas", client.queue().await?.len());
    println!(
        "Raíz de la biblioteca: {} entradas",
        client.browse("").await?.len()
    );

    let artists = client.list_tag(Tag::Artist, &[]).await?;
    println!("Artistas: {}", artists.len());
    if let Some(artist) = artists.iter().find(|artist| !artist.is_empty()) {
        let albums = client
            .list_tag(Tag::Album, &[TagFilter::new(Tag::Artist, artist)])
            .await?;
        println!("  {artist}: {} álbumes", albums.len());
    }

    let found = client
        .search(Tag::Any, &SearchText::new(SAMPLE_SEARCH)?)
        .await?;
    println!("Búsqueda {SAMPLE_SEARCH:?}: {} resultados", found.len());

    let playlists = client.playlists().await?;
    println!("Playlists guardadas: {}", playlists.len());
    if let Some(first) = playlists.first() {
        let songs = client.playlist_songs(first).await?;
        println!("  {first}: {} temas", songs.len());
    }

    for partition in client.partitions().await? {
        println!("Partición {partition}:");
        for output in client.outputs(&partition).await? {
            let state = if output.enabled { "activa" } else { "inactiva" };
            println!(
                "  [{}] {} ({}, {state})",
                output.id, output.name, output.plugin
            );
        }
    }
    Ok(())
}

/// El host y el puerto de los argumentos, o lo que resuelve la configuración.
fn server_from_arguments() -> anyhow::Result<(AddressCandidates, ConnectionSettings)> {
    let mut arguments = std::env::args().skip(1);
    let Some(host) = arguments.next() else {
        let settings = config::load()?;
        return Ok((settings.addresses, settings.connection));
    };
    let port = match arguments.next() {
        Some(port) => port.parse()?,
        None => DEFAULT_PORT,
    };
    let address = Address::new(&host, port);
    Ok((
        AddressCandidates::configured(address),
        ConnectionSettings::default(),
    ))
}
