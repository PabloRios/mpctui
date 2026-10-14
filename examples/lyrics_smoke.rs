//! Letras de solo lectura: toma el tema actual de MPD (`currentsong`) y consulta LRCLIB,
//! sin guardar nada en la caché. Imprime si la encontró, si es sincronizada y las
//! primeras líneas. No modifica MPD.
//! Uso: `cargo run --example lyrics_smoke` (servidor y contraseña salen de `config.toml`).

use anyhow::{Context, bail};
use mpctui::domain::{FetchedLyrics, LyricsQuery, LyricsSource};
use mpctui::factory::connect_mpd;
use mpctui::repository::mpd::first_reachable;
use mpctui::repository::{LRCLIB_URL, LrclibProvider, LyricsProvider, PlayerRepository};
use mpctui::service::parse_lrc;

/// Líneas de la letra que se muestran.
const PREVIEW_LINES: usize = 8;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = mpctui::config::load()?;
    let address = first_reachable(&settings.addresses, &settings.connection).await?;
    let mut mpd = connect_mpd(&address, &settings.connection).await?;
    let song = mpd
        .current_song()
        .await?
        .context("no hay tema actual en MPD")?;
    let Some(track) = LyricsQuery::for_song(&song).track else {
        bail!("al tema {} le faltan el artista o el título", song.file);
    };
    println!(
        "Buscando en LRCLIB: {} — {} (álbum {:?}, {:?})",
        track.artist(),
        track.title(),
        track.album(),
        track.duration()
    );
    let mut provider = LrclibProvider::new(LRCLIB_URL)?;
    match provider.fetch(&track).await? {
        FetchedLyrics::Text(text) => {
            let lyrics = parse_lrc(&text, LyricsSource::Lrclib);
            let timing = if lyrics.is_synced() {
                "sincronizada"
            } else {
                "texto"
            };
            println!("Encontrada ({timing}, {} líneas):", lyrics.lines.len());
            for line in lyrics.lines.iter().take(PREVIEW_LINES) {
                let time = line
                    .time
                    .map(|time| format!("{:>6.2}s ", time.as_secs_f32()))
                    .unwrap_or_default();
                println!("  {time}{}", line.text);
            }
        }
        FetchedLyrics::Instrumental => println!("LRCLIB dice que es instrumental."),
        FetchedLyrics::Missing => println!("LRCLIB no tiene letra para este tema."),
    }
    Ok(())
}
