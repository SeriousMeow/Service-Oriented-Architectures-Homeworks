mod generated;
pub use generated::warehouse;

mod api;
mod config;
mod event;
mod generator;
mod publisher;
mod schema;

use crate::api::{AppState, router};
use crate::config::Config;
use crate::publisher::Publisher;
use anyhow::Context;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()
        .map_err(anyhow::Error::msg)
        .context("invalid configuration")?;
    let publisher = Publisher::connect(&cfg)
        .await
        .map_err(anyhow::Error::msg)
        .context("kafka / schema registry")?;

    if cfg.mode.run_generator() {
        let publisher_clone = Arc::clone(&publisher);
        let gen_cfg = cfg.clone();
        tokio::spawn(async move {
            generator::run(&gen_cfg, publisher_clone).await;
        });
    }

    let state = AppState {
        publisher,
        config: cfg.clone(),
    };
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, mode = ?cfg.mode, "warehouse-producer listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
