//! Visualizador de solo lectura: lee la salida fifo de MPD unos segundos con la forma
//! Barras y otros tantos con Medidores, e imprime el nivel máximo de cada banda, el RMS
//! y el pico por canal y la correlación. Sirve para ajustar `FLOOR_DB` y `CEIL_DB`
//! mientras suena música. No envía ningún comando a MPD.
//! Uso: `cargo run --example spectrum_smoke` (ruta y formato salen de `config.toml`).

use std::time::Duration;

use anyhow::bail;
use mpctui::domain::{
    ChannelLevel, METER_FLOOR_DB, VisualContent, VisualFrame, VisualRequest, VisualizerStyle,
    band_centers,
};
use mpctui::factory::visualizer_launcher;
use mpctui::service::{CEIL_DB, FLOOR_DB, RunningVisualizer, VisualState};

/// Cuánto se escucha cada forma.
const LISTEN: Duration = Duration::from_secs(3);
const BANDS: usize = 32;
/// Columnas de la barra de texto de un nivel completo.
const BAR_COLUMNS: f32 = 50.0;
const CHANNEL_NAMES: [&str; 2] = ["L", "R"];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = mpctui::config::load()?;
    let launcher = visualizer_launcher(&settings);
    let source = launcher.source_label().to_owned();
    println!(
        "Leyendo {source} ({}) durante {} s por forma; rango {FLOOR_DB} a {CEIL_DB} dB",
        launcher.format(),
        LISTEN.as_secs()
    );
    let bars = VisualRequest {
        style: VisualizerStyle::Bars,
        columns: BANDS,
        rows: 0,
    };
    let mut running = launcher.launch(bars);
    let mut maximums = vec![0.0_f32; BANDS];
    let (frames, with_signal) = listen(&mut running, &source, |frame| {
        if let VisualContent::Bars(bands) = &frame.content {
            for (maximum, level) in maximums.iter_mut().zip(&bands.levels) {
                *maximum = maximum.max(*level);
            }
        }
    })
    .await?;
    println!("{frames} cuadros, {with_signal} con señal. Máximo por banda:");
    for (center, level) in band_centers(BANDS).iter().zip(&maximums) {
        let bar = "#".repeat((level * BAR_COLUMNS).round() as usize);
        println!("{center:>8.0} Hz  {level:.2}  {bar}");
    }

    running.set_request(VisualRequest {
        style: VisualizerStyle::Meters,
        ..bars
    });
    let mut loudest: Vec<ChannelLevel> = Vec::new();
    let mut last_correlation = None;
    listen(&mut running, &source, |frame| {
        if let VisualContent::Meters {
            channels,
            correlation,
        } = &frame.content
        {
            loudest.resize(channels.len(), ChannelLevel::SILENT);
            for (maximum, level) in loudest.iter_mut().zip(channels) {
                maximum.rms = maximum.rms.max(level.rms);
                maximum.peak = maximum.peak.max(level.peak);
            }
            last_correlation = *correlation;
        }
    })
    .await?;
    println!("Máximo por canal (dBFS, piso {METER_FLOOR_DB}):");
    for (name, level) in CHANNEL_NAMES.iter().zip(&loudest) {
        println!("  {name}  RMS {:6.1}  pico {:6.1}", level.rms, level.peak);
    }
    if let Some(correlation) = last_correlation {
        println!("Correlación al final: {correlation:+.2}");
    }
    Ok(())
}

/// Recorre los cuadros durante `LISTEN`; devuelve cuántos llegaron y cuántos con señal.
async fn listen(
    running: &mut RunningVisualizer,
    source: &str,
    mut each: impl FnMut(&VisualFrame),
) -> anyhow::Result<(usize, usize)> {
    let (mut frames, mut with_signal) = (0, 0);
    let deadline = tokio::time::sleep(LISTEN);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            () = &mut deadline => return Ok((frames, with_signal)),
            state = running.next_state() => match state {
                Some(VisualState::Frame(frame)) => {
                    frames += 1;
                    with_signal += usize::from(frame.signal);
                    each(&frame);
                }
                Some(VisualState::Unavailable(reason)) => {
                    bail!("No se puede leer {source}: {reason}")
                }
                None => return Ok((frames, with_signal)),
            }
        }
    }
}
