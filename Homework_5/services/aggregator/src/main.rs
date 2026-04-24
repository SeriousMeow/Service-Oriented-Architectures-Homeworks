mod aggregate;
mod api;
mod clickhouse;
mod config;
mod pg;

use crate::api::{router, run_cycle, AppState};
use crate::clickhouse::ClickhouseHttp;
use crate::config::Config;
use tokio::signal;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env().map_err(anyhow::Error::msg)?;
    let ch = ClickhouseHttp::new(
        cfg.clickhouse_url.clone(),
        cfg.clickhouse_user.clone(),
        cfg.clickhouse_password.clone(),
    )?;
    let pg_pool = pg::create_pool().map_err(anyhow::Error::msg)?;

    let state = AppState {
        ch: ch.clone(),
        pg: pg_pool.clone(),
    };

    let interval = cfg.aggregation_interval;
    let sched_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        loop {
            let d = aggregate::yesterday_utc();
            let started = std::time::Instant::now();
            tracing::info!(metric_date = %d, "aggregation cycle started (scheduled)");
            match run_cycle(&sched_state, d).await {
                Ok(r) => {
                    tracing::info!(
                        metric_date = %r.metric_date,
                        records_processed = r.records_processed,
                        duration_ms = r.duration_ms,
                        metrics_written = r.metrics_written,
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "aggregation cycle finished (scheduled)"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        metric_date = %d,
                        "aggregation cycle failed (scheduled)"
                    );
                }
            }
            tokio::time::sleep(interval).await;
        }
    });

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, interval_sec = interval.as_secs(), "aggregator listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
