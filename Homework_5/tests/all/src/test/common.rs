use anyhow::{bail, Context, Result};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use aws_types::region::Region;
use chrono::{Days, NaiveDate, Utc};
use tokio_postgres::NoTls;

#[derive(Debug)]
pub struct Config {
    pub schema_registry_url: String,
    pub producer_url: String,
    pub aggregator_url: String,
    pub exporter_url: String,
    pub clickhouse_http: String,
    pub clickhouse_user: String,
    pub clickhouse_password: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_db: String,
    pub postgres_user: String,
    pub postgres_password: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub aws_access_key_id: String,
    pub aws_secret_access_key: String,
    pub minio_bucket: String,
}

pub fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

pub fn build_config() -> Result<Config> {
    Ok(Config {
        schema_registry_url: env_or_default("SCHEMA_REGISTRY_URL", "http://schema-registry:8081"),
        producer_url: env_or_default("PRODUCER_URL", "http://producer:8080"),
        aggregator_url: env_or_default("AGGREGATOR_URL", "http://aggregator:8081"),
        exporter_url: env_or_default("EXPORTER_URL", "http://exporter:8082"),
        clickhouse_http: env_or_default("CLICKHOUSE_HTTP", "http://clickhouse:8123"),
        clickhouse_user: env_or_default("CLICKHOUSE_USER", "default"),
        clickhouse_password: env_or_default("CLICKHOUSE_PASSWORD", "movie"),
        postgres_host: env_or_default("POSTGRES_HOST", "postgres"),
        postgres_port: env_or_default("POSTGRES_PORT", "5432")
            .parse()
            .context("parse POSTGRES_PORT as u16")?,
        postgres_db: env_or_default("POSTGRES_DB", "movie_analytics"),
        postgres_user: env_or_default("POSTGRES_USER", "movie"),
        postgres_password: env_or_default("POSTGRES_PASSWORD", "movie"),
        s3_endpoint: env_or_default("S3_ENDPOINT", "http://minio:9000"),
        s3_region: env_or_default("S3_REGION", "us-east-1"),
        aws_access_key_id: env_or_default("AWS_ACCESS_KEY_ID", "minio"),
        aws_secret_access_key: env_or_default("AWS_SECRET_ACCESS_KEY", "minio12345"),
        minio_bucket: env_or_default("S3_BUCKET", "movie-analytics"),
    })
}

pub fn yesterday_utc() -> String {
    let today = Utc::now().date_naive();
    today
        .checked_sub_days(Days::new(1))
        .unwrap_or(today)
        .format("%Y-%m-%d")
        .to_string()
}

pub async fn run_clickhouse_query(client: &reqwest::Client, cfg: &Config, query: &str) -> Result<String> {
    let url = format!("{}/", cfg.clickhouse_http.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .basic_auth(&cfg.clickhouse_user, Some(&cfg.clickhouse_password))
        .body(query.to_string())
        .send()
        .await
        .with_context(|| format!("clickhouse query failed: {}", query))?;
    if !resp.status().is_success() {
        bail!(
            "clickhouse query HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );
    }
    resp.text().await.context("read clickhouse response body")
}

pub async fn query_postgres_count(cfg: &Config, date: &str) -> Result<i64> {
    let parsed_date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .with_context(|| format!("parse metric date '{}' as YYYY-MM-DD", date))?;
    let conn_str = format!(
        "host={} port={} user={} password={} dbname={}",
        cfg.postgres_host, cfg.postgres_port, cfg.postgres_user, cfg.postgres_password, cfg.postgres_db
    );
    let (pg_client, connection) = tokio_postgres::connect(&conn_str, NoTls)
        .await
        .context("connect to postgres")?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let row = pg_client
        .query_one(
            "SELECT count(*)::bigint FROM metrics WHERE metric_date = $1",
            &[&parsed_date],
        )
        .await
        .context("query metrics count in postgres")?;
    Ok(row.get::<usize, i64>(0))
}

pub async fn build_s3_client(cfg: &Config) -> Result<S3Client> {
    let creds = Credentials::new(
        &cfg.aws_access_key_id,
        &cfg.aws_secret_access_key,
        None,
        None,
        "all-test-static",
    );
    let conf = aws_config::defaults(BehaviorVersion::latest())
        .endpoint_url(&cfg.s3_endpoint)
        .region(Region::new(cfg.s3_region.clone()))
        .credentials_provider(creds)
        .load()
        .await;
    let s3_conf = aws_sdk_s3::config::Builder::from(&conf)
        .force_path_style(true)
        .build();
    Ok(S3Client::from_conf(s3_conf))
}
