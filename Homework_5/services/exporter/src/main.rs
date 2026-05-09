mod api;
mod config;
mod export_job;
mod pg;

use crate::api::{router, AppState};
use export_job::{
    build_s3_client, ensure_bucket, export_date_to_s3, yesterday_utc,
};
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = config::Config::from_env().map_err(anyhow::Error::msg)?;
    let pg_pool = pg::create_pool().map_err(anyhow::Error::msg)?;
    let s3 = build_s3_client(&cfg).await?;
    ensure_bucket(&s3, &cfg.s3_bucket).await?;

    let state = AppState {
        pg: pg_pool.clone(),
        s3: s3.clone(),
        bucket: cfg.s3_bucket.clone(),
    };

    let sched_state = state.clone();
    let interval = cfg.export_interval;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        loop {
            let d = yesterday_utc();
            tracing::info!(metric_date = %d, "export cycle started (scheduled)");
            match export_date_to_s3(&sched_state.pg, &sched_state.s3, &sched_state.bucket, d).await {
                Ok(()) => tracing::info!(metric_date = %d, "export cycle finished (scheduled)"),
                Err(e) => tracing::error!(
                    error = %e,
                    metric_date = %d,
                    "export cycle failed (scheduled); will retry on next interval"
                ),
            }
            tokio::time::sleep(interval).await;
        }
    });

    let app = router(state);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!(addr = %cfg.bind_addr, interval_sec = interval.as_secs(), "exporter listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
