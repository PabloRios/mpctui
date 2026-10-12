//! Prueba de humo de solo lectura contra un MPD real.
//! Uso: `cargo run --example smoke [host] [port]` (por defecto localhost 6600).

use mpctui::domain::{SearchText, Tag, TagFilter};
use mpctui::factory::connect_mpd;
use mpctui::repository::mpd::{Address, DEFAULT_HOST, DEFAULT_PORT};
use mpctui::repository::{
    LibraryRepository, PlayerRepository, PlaylistRepository, QueueRepository,
};

const SAMPLE_SEARCH: &str = "love";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let address = address_from_arguments()?;
    let mut client = connect_mpd(&address, None).await?;
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
    Ok(())
}

fn address_from_arguments() -> anyhow::Result<Address> {
    let mut arguments = std::env::args().skip(1);
    let host = arguments.next().unwrap_or_else(|| DEFAULT_HOST.into());
    let port = match arguments.next() {
        Some(port) => port.parse()?,
        None => DEFAULT_PORT,
    };
    Ok(Address::new(&host, port))
}
