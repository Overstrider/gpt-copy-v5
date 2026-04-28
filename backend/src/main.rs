use std::process::ExitCode;

use gpt_copy_v5_backend::{build_app, config::Config};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::from_filename("../.env").ok();
    dotenvy::dotenv().ok();
    init_tracing();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = %error, "backend failed");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let bind_addr = config.bind_addr;
    let app = build_app(config).await?;
    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!(%bind_addr, "gpt-copy-v5 backend listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
