use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct PostEventResponse {
    event_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct ChRow {
    event_id: String,
    user_id: String,
    movie_id: String,
    event_type: String,
    #[serde(deserialize_with = "deserialize_ts_millis")]
    ts_millis: i64,
    device_type: String,
    session_id: String,
    progress_seconds: i32,
}

fn deserialize_ts_millis<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| D::Error::custom("ts_millis number out of range")),
        serde_json::Value::String(s) => s.parse().map_err(D::Error::custom),
        _ => Err(D::Error::custom("ts_millis must be a number or string")),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let started_at = Instant::now();
    let producer_base = std::env::var("PRODUCER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let ch_base =
        std::env::var("CLICKHOUSE_HTTP").unwrap_or_else(|_| "http://127.0.0.1:8123".to_string());
    let ch_user = std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string());
    let ch_password = std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_else(|_| "movie".to_string());

    let event_id = Uuid::new_v4();
    let session_id = format!("itest-session-{}", Uuid::new_v4());
    let user_id = format!("itest-user-{}", Uuid::new_v4());
    let movie_id = "itest-movie-alpha".to_string();
    let ts: DateTime<Utc> = Utc::now();
    let body = serde_json::json!({
        "event_id": event_id,
        "user_id": user_id,
        "movie_id": movie_id,
        "event_type": "VIEW_STARTED",
        "timestamp": ts.to_rfc3339_opts(SecondsFormat::Millis, true),
        "device_type": "DESKTOP",
        "session_id": session_id,
        "progress_seconds": 0
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("http client")?;

    let post_url = format!("{}/events", producer_base.trim_end_matches('/'));
    let resp = client
        .post(&post_url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("POST {}", post_url))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("producer returned {}: {}", status, text);
    }

    let parsed: PostEventResponse = resp
        .json()
        .await
        .context("decode producer JSON")?;
    if parsed.event_id != event_id {
        bail!(
            "producer echoed unexpected event_id: expected {}, got {}",
            event_id,
            parsed.event_id
        );
    }

    let ch_url = format!(
        "{}/?default_format=JSONEachRow",
        ch_base.trim_end_matches('/')
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut last_err: Option<anyhow::Error> = None;

    while tokio::time::Instant::now() < deadline {
        let q = format!(
            "SELECT event_id, user_id, movie_id, event_type, toUnixTimestamp64Milli(timestamp) AS ts_millis, device_type, session_id, progress_seconds FROM movie_events WHERE event_id = '{}' LIMIT 1",
            event_id
        );

        let r = client
            .post(&ch_url)
            .basic_auth(&ch_user, Some(&ch_password))
            .body(q)
            .send()
            .await
            .context("clickhouse query");

        match r {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.context("clickhouse body")?;
                let line = text.lines().find(|l| !l.trim().is_empty());
                if let Some(line) = line {
                    let row: ChRow = serde_json::from_str(line).context("parse CH JSON row")?;
                    assert_row(&row, &event_id, &user_id, &movie_id, &session_id, ts, 0)?;
                    println!(
                        "integration OK: event_id={} present in movie_events",
                        event_id
                    );
                    println!("integration duration: {:.3}s", started_at.elapsed().as_secs_f64());
                    return Ok(());
                }
            }
            Ok(resp) => {
                last_err = Some(anyhow::anyhow!(
                    "clickhouse HTTP {}: {}",
                    resp.status(),
                    resp.text().await.unwrap_or_default()
                ));
            }
            Err(e) => last_err = Some(e.into()),
        }

        tokio::time::sleep(Duration::from_millis(750)).await;
    }

    if let Some(e) = last_err {
        bail!("timed out waiting for row in ClickHouse: {:?}", e);
    }
    bail!("timed out waiting for row in ClickHouse (no error recorded)");
}

fn assert_row(
    row: &ChRow,
    event_id: &Uuid,
    user_id: &str,
    movie_id: &str,
    session_id: &str,
    expected_ts: DateTime<Utc>,
    progress: i32,
) -> Result<()> {
    if row.event_id != event_id.to_string() {
        bail!("event_id mismatch: {} vs {}", row.event_id, event_id);
    }
    if row.user_id != user_id {
        bail!("user_id mismatch");
    }
    if row.movie_id != movie_id {
        bail!("movie_id mismatch");
    }
    if row.event_type != "VIEW_STARTED" {
        bail!("event_type mismatch: {}", row.event_type);
    }
    if row.device_type != "DESKTOP" {
        bail!("device_type mismatch: {}", row.device_type);
    }
    if row.session_id != session_id {
        bail!("session_id mismatch");
    }
    if row.progress_seconds != progress {
        bail!("progress_seconds mismatch");
    }

    let expected_ms = expected_ts.timestamp_millis();
    let delta = (row.ts_millis - expected_ms).abs();
    if delta > 1500 {
        bail!(
            "timestamp skew too large: CH {} ms vs expected {} ms (delta {} ms)",
            row.ts_millis,
            expected_ms,
            delta
        );
    }

    Ok(())
}
