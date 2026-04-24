use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use deadpool_postgres::{ManagerConfig, Pool, RecyclingMethod, Runtime};
use serde::Serialize;
use tokio_postgres::NoTls;

const FETCH_SQL: &str = r#"
SELECT metric_name, metric_value, metric_payload, computed_at
FROM metrics
WHERE metric_date = $1
ORDER BY metric_name
"#;

#[derive(Debug, Serialize)]
pub struct MetricRow {
    pub metric_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric_payload: Option<serde_json::Value>,
    pub computed_at: DateTime<Utc>,
}

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

pub async fn fetch_metrics_for_date(
    pool: &Pool,
    metric_date: NaiveDate,
) -> anyhow::Result<Vec<MetricRow>> {
    let client = pool.get().await?;
    let rows = client.query(FETCH_SQL, &[&metric_date]).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let name: String = r.get(0);
        let value: Option<f64> = r.get(1);
        let payload: Option<serde_json::Value> = r.get(2);
        let computed_at: DateTime<FixedOffset> = r.get(3);
        let computed_at = computed_at.with_timezone(&Utc);
        out.push(MetricRow {
            metric_name: name,
            metric_value: value,
            metric_payload: payload,
            computed_at,
        });
    }
    Ok(out)
}
