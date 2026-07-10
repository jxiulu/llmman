mod llm;
mod model;
mod cli;
mod save_data;

use color_eyre as eyre;
use ratatui::backend::CrosstermBackend;

use std::io;

use crate::save_data::config::{self, Config};

#[derive(Default)]
pub struct AppData {
    config: Config
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _guard = cli::setup_tracing_subscriber();

    cli::setup()?;

    let config: Config = match config::read_config() {
        Ok(c) => c,
        Err(e) => {
            config::create_config()?
        },
    };

    let appdata = AppData { config };

    let backend = CrosstermBackend::new(io::stdout());
    let mut app = cli::App::new(backend, appdata)?;

    app.ready_terminal()?;
    let result = app.run().await;
    app.unready_terminal()?;

    result
}
