mod api;
mod config;
mod event;
mod generator;
mod publisher;

use crate::api::{router, AppState};
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

    let cfg = Config::from_env().map_err(anyhow::Error::msg).context("invalid configuration")?;
    let publisher = Publisher::connect(
        &cfg.kafka_bootstrap,
        cfg.movie_events_topic.clone(),
        &cfg.schema_registry_url,
        &cfg.schema_subject,
    )
    .await
    .map_err(anyhow::Error::msg)
    .context("kafka / schema registry")?;

    if cfg.mode.run_generator() {
        let p = Arc::clone(&publisher);
        let gen_cfg = cfg.clone();
        tokio::spawn(async move {
            generator::run(&gen_cfg, p).await;
        });
    }

    let state = AppState { publisher };
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, mode = ?cfg.mode, "producer listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
