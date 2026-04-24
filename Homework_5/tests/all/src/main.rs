use anyhow::{Context, Result};
use std::time::{Duration, Instant};
mod test;

use test::common::{build_config, build_s3_client, env_or_default, yesterday_utc};

#[tokio::main]
async fn main() -> Result<()> {
    let started = Instant::now();
    let cfg = build_config()?;
    let schema_subject = env_or_default("SCHEMA_REGISTRY_SUBJECT", "movie-events-value");
    let metric_date = yesterday_utc();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("build HTTP client")?;

    let s3_client = build_s3_client(&cfg).await?;

    test::schema_registry::verify_subject(&client, &cfg, &schema_subject).await?;
    let event_id = test::producer_raw::send_event(&client, &cfg).await?;
    test::producer_raw::wait_event_in_clickhouse(&client, &cfg, event_id, 60).await?;
    test::aggregation::recompute_and_verify(&client, &cfg, &metric_date).await?;
    test::exporter::export_and_verify(&client, &s3_client, &cfg, &metric_date).await?;
    test::views::verify_clickhouse_views(&client, &cfg).await?;

    println!("All all-test checks passed.");
    println!("Duration: {:.3}s", started.elapsed().as_secs_f64());
    Ok(())
}
