use anyhow::Context;
use crossterm::event::EventStream;
use mpctui::{config, factory, logging};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init()?;
    let settings = config::load()?;
    let app = factory::build_app(&settings)
        .await
        .with_context(|| format!("no se pudo conectar a MPD en {}", settings.address))?;

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal, EventStream::new()).await;
    ratatui::restore();
    Ok(result?)
}
