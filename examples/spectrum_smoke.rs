//! Espectro de solo lectura: lee la salida fifo de MPD durante unos segundos e imprime
//! el nivel máximo de cada banda. Sirve para ajustar `FLOOR_DB` y `CEIL_DB` mientras
//! suena música. No envía ningún comando a MPD.
//! Uso: `cargo run --example spectrum_smoke` (ruta y formato salen de `config.toml`).

use std::time::Duration;

use anyhow::bail;
use mpctui::domain::band_centers;
use mpctui::factory::visualizer_launcher;
use mpctui::service::{CEIL_DB, FLOOR_DB, SpectrumState};

const LISTEN: Duration = Duration::from_secs(3);
const BANDS: usize = 32;
/// Columnas de la barra de texto de un nivel completo.
const BAR_COLUMNS: f32 = 50.0;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = mpctui::config::load()?;
    let launcher = visualizer_launcher(&settings);
    println!(
        "Leyendo {} ({}) durante {} s; rango {FLOOR_DB} a {CEIL_DB} dB",
        launcher.source_label(),
        launcher.format(),
        LISTEN.as_secs()
    );
    let mut running = launcher.launch(BANDS);
    let mut maximums = vec![0.0_f32; BANDS];
    let (mut frames, mut with_signal) = (0, 0);
    let deadline = tokio::time::sleep(LISTEN);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            () = &mut deadline => break,
            state = running.next_state() => match state {
                Some(SpectrumState::Frame(frame)) => {
                    frames += 1;
                    with_signal += usize::from(frame.signal);
                    for (maximum, level) in maximums.iter_mut().zip(&frame.levels) {
                        *maximum = maximum.max(*level);
                    }
                }
                Some(SpectrumState::Unavailable(reason)) => {
                    bail!("No se puede leer {}: {reason}", launcher.source_label())
                }
                None => break,
            }
        }
    }
    println!("{frames} cuadros, {with_signal} con señal. Máximo por banda:");
    for (center, level) in band_centers(BANDS).iter().zip(&maximums) {
        let bar = "#".repeat((level * BAR_COLUMNS).round() as usize);
        println!("{center:>8.0} Hz  {level:.2}  {bar}");
    }
    Ok(())
}
