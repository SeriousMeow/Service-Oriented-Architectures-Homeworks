use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

use crate::test::common::{run_clickhouse_query, Config};

#[derive(Debug, Deserialize)]
struct ProducerResponse {
    event_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct ChCountRow {
    #[serde(deserialize_with = "de_i64_from_string_or_number")]
    count: i64,
}

pub async fn send_event(client: &reqwest::Client, cfg: &Config) -> Result<Uuid> {
    let event_id = Uuid::new_v4();
    let session_id = Uuid::new_v4().to_string();
    let ts: DateTime<Utc> = Utc::now();
    let payload = serde_json::json!({
        "event_id": event_id,
        "user_id": "all-test-user",
        "movie_id": "movie-42",
        "event_type": "VIEW_STARTED",
        "timestamp": ts.to_rfc3339_opts(SecondsFormat::Millis, true),
        "device_type": "DESKTOP",
        "session_id": session_id,
        "progress_seconds": 0
    });
    let produce_url = format!("{}/events", cfg.producer_url.trim_end_matches('/'));
    let produce_resp = client
        .post(&produce_url)
        .json(&payload)
        .send()
        .await
        .with_context(|| format!("POST {}", produce_url))?;
    if !produce_resp.status().is_success() {
        bail!(
            "producer API failed: {}",
            produce_resp.text().await.unwrap_or_default()
        );
    }
    let parsed: ProducerResponse = produce_resp
        .json()
        .await
        .context("parse producer response")?;
    if parsed.event_id != event_id {
        bail!(
            "producer returned unexpected event_id: expected {}, got {}",
            event_id,
            parsed.event_id
        );
    }
    Ok(event_id)
}

pub async fn wait_event_in_clickhouse(
    client: &reqwest::Client,
    cfg: &Config,
    event_id: Uuid,
    tries: usize,
) -> Result<()> {
    for _ in 0..tries {
        let q = format!(
            "SELECT count() AS count FROM movie_events WHERE event_id = '{}' FORMAT JSONEachRow",
            event_id
        );
        let body = run_clickhouse_query(client, cfg, &q).await?;
        let row: ChCountRow =
            serde_json::from_str(body.trim()).context("parse clickhouse count row")?;
        if row.count == 1 {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    bail!("event {} not found in ClickHouse raw table", event_id);
}

fn de_i64_from_string_or_number<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| D::Error::custom("numeric value out of range")),
        serde_json::Value::String(s) => s.parse::<i64>().map_err(D::Error::custom),
        _ => Err(D::Error::custom("expected string or number")),
    }
}
