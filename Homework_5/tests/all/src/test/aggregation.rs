use anyhow::{bail, Context, Result};

use crate::test::common::{query_postgres_count, Config};

pub async fn recompute_and_verify(
    client: &reqwest::Client,
    cfg: &Config,
    metric_date: &str,
) -> Result<i64> {
    let recompute_url = format!("{}/recompute", cfg.aggregator_url.trim_end_matches('/'));
    let recompute_resp = client
        .post(&recompute_url)
        .json(&serde_json::json!({ "date": metric_date }))
        .send()
        .await
        .with_context(|| format!("POST {}", recompute_url))?;
    if !recompute_resp.status().is_success() {
        bail!(
            "aggregator recompute failed: {}",
            recompute_resp.text().await.unwrap_or_default()
        );
    }

    let pg_count = query_postgres_count(cfg, metric_date).await?;
    if pg_count <= 0 {
        bail!("no metrics found in postgres for date {}", metric_date);
    }
    Ok(pg_count)
}
