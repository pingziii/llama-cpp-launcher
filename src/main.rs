mod config;
mod error;
mod hardware;
mod launcher;
mod params;
mod tui;

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (TUI uses stdout)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("TUI LLM Launcher starting");

    // Load or create configuration
    let config = config::load_or_create_config()?;

    info!(
        model_dir = %config.model_dir,
        port = config.port,
        "Configuration loaded"
    );

    // Initialize and run the TUI application
    let mut app = tui::app::App::new(config);
    app.run().await?;

    info!("TUI LLM Launcher shutting down");
    Ok(())
}
