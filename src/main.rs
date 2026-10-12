use anyhow::anyhow;
use crossterm::event::EventStream;
use mpctui::i18n::{self, Message};
use mpctui::{config, factory, logging};

/// Los errores de arranque se explican en el idioma configurado (o en el de por
/// defecto si la configuración no se pudo leer).
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _log_guard = logging::init()?;
    let settings = config::load().map_err(|failure| {
        let texts = i18n::catalog(failure.language);
        anyhow!(texts.message(&Message::Config(&failure.error)))
    })?;
    let app = factory::build_app(&settings).await.map_err(|error| {
        let texts = i18n::catalog(settings.language);
        anyhow!(texts.message(&Message::ConnectFailed {
            address: &settings.address.to_string(),
            error: &error,
        }))
    })?;

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal, EventStream::new()).await;
    ratatui::restore();
    Ok(result?)
}
