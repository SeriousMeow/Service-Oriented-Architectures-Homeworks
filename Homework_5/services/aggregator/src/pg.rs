use deadpool_postgres::{ManagerConfig, Pool, RecyclingMethod, Runtime};
use serde_json::Value;
use tokio_postgres::types::Json;
use tokio_postgres::NoTls;

const UPSERT_SQL: &str = r#"
INSERT INTO metrics (metric_date, metric_name, metric_value, metric_payload, computed_at)
VALUES ($1::date, $2, $3, $4, $5)
ON CONFLICT (metric_date, metric_name) DO UPDATE SET
    metric_value = EXCLUDED.metric_value,
    metric_payload = EXCLUDED.metric_payload,
    computed_at = EXCLUDED.computed_at
"#;

pub fn create_pool() -> Result<Pool, Box<dyn std::error::Error + Send + Sync>> {
    let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = std::env::var("POSTGRES_PORT")
        .unwrap_or_else(|_| "5432".to_string())
        .parse()?;
    let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "movie".to_string());
    let password = std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "movie".to_string());
    let database = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "movie_analytics".to_string());

    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some(host);
    cfg.port = Some(port);
    cfg.user = Some(user);
    cfg.password = Some(password);
    cfg.dbname = Some(database);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: 8,
        ..Default::default()
    });

    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}

pub async fn upsert_metric(
    pool: &Pool,
    metric_date: &str,
    name: &str,
    value: Option<f64>,
    payload: Option<Value>,
    computed_at: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<()> {
    const MAX_ATTEMPTS: u32 = 4;
    let mut delay_ms = 100u64;
    let mut last_err = None::<anyhow::Error>;

    let metric_date_parsed = chrono::NaiveDate::parse_from_str(metric_date, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("invalid metric_date '{metric_date}': {e}"))?;

    for attempt in 0..MAX_ATTEMPTS {
        let client = match pool.get().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, attempt, metric_date, metric_name = name, "postgres pool get failed (upsert)");
                last_err = Some(anyhow::anyhow!(e));
                if attempt + 1 == MAX_ATTEMPTS {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(2000);
                continue;
            }
        };

        let payload_json: Option<Json<Value>> = payload.as_ref().map(|p| Json(p.clone()));
        let res = client
            .execute(
                UPSERT_SQL,
                &[&metric_date_parsed, &name, &value, &payload_json, &computed_at],
            )
            .await;

        match res {
            Ok(_) => return Ok(()),
            Err(e) => {
                tracing::warn!(error = %e, attempt, metric_date, metric_name = name, "postgres upsert failed");
                last_err = Some(anyhow::Error::new(e));
                if attempt + 1 == MAX_ATTEMPTS {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(2000);
            }
        }
    }

    tracing::error!(
        metric_date,
        metric_name = name,
        "postgres upsert failed after retries"
    );
    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!("postgres upsert exhausted retries")
    }))
}
