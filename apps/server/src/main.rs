mod config;
mod hub;
mod store;

use anyhow::Context;
use clap::Parser;
use config::ServerConfig;
use hub::Hub;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = config::Args::parse();
    let config = ServerConfig::load(args).context("load config")?;
    let hub = Arc::new(Hub::open(config.clone()).context("open hub")?);
    let app = hub.clone().router();
    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .with_context(|| format!("bind {}", config.addr))?;

    info!(
        addr = %config.addr,
        db = %config.db_path.display(),
        version = env!("CARGO_PKG_VERSION"),
        "codux service listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve codux service")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
