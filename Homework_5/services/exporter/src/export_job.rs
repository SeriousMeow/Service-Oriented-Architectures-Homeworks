use crate::pg::{self, MetricRow};
use anyhow::Context;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use aws_types::region::Region;
use chrono::{Days, NaiveDate, Utc};
use deadpool_postgres::Pool;
use serde::Serialize;

#[derive(Serialize)]
pub struct DailyExportBody {
    pub metric_date: String,
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub metrics: Vec<MetricRow>,
}

pub async fn build_s3_client(cfg: &crate::config::Config) -> anyhow::Result<S3Client> {
    let creds = Credentials::new(
        &cfg.aws_access_key,
        &cfg.aws_secret_key,
        None,
        None,
        "static",
    );
    let loader = aws_config::defaults(BehaviorVersion::latest())
        .endpoint_url(&cfg.s3_endpoint)
        .region(Region::new(cfg.s3_region.clone()))
        .credentials_provider(creds);
    let conf = loader.load().await;
    let s3_conf = aws_sdk_s3::config::Builder::from(&conf)
        .force_path_style(true)
        .build();
    Ok(S3Client::from_conf(s3_conf))
}

pub async fn ensure_bucket(client: &S3Client, bucket: &str) -> anyhow::Result<()> {
    match client.head_bucket().bucket(bucket).send().await {
        Ok(_) => {
            tracing::info!(bucket, "s3 bucket exists");
            Ok(())
        }
        Err(e) => {
            tracing::info!(bucket, error = %e, "head_bucket failed; attempting create_bucket");
            client
                .create_bucket()
                .bucket(bucket)
                .send()
                .await
                .context("create_bucket")?;
            tracing::info!(bucket, "s3 bucket created");
            Ok(())
        }
    }
}

pub fn object_key(metric_date: &str) -> String {
    format!("daily/{metric_date}/aggregates.json")
}

pub async fn export_date_to_s3(
    pg_pool: &Pool,
    s3: &S3Client,
    bucket: &str,
    metric_date: NaiveDate,
) -> anyhow::Result<()> {
    let date_str = metric_date.format("%Y-%m-%d").to_string();
    let metrics = pg::fetch_metrics_for_date(pg_pool, metric_date)
        .await
        .with_context(|| format!("postgres fetch metrics for {date_str}"))?;

    let body = DailyExportBody {
        metric_date: date_str.clone(),
        exported_at: Utc::now(),
        metrics,
    };
    let json = serde_json::to_vec_pretty(&body).context("serialize export json")?;
    let key = object_key(&date_str);

    s3.put_object()
        .bucket(bucket)
        .key(&key)
        .content_type("application/json")
        .body(json.into())
        .send()
        .await
        .with_context(|| format!("s3 put_object s3://{bucket}/{key}"))?;

    tracing::info!(
        bucket,
        key,
        metric_date = %date_str,
        metric_count = body.metrics.len(),
        "daily export uploaded (overwrite semantics)"
    );
    Ok(())
}

pub fn yesterday_utc() -> NaiveDate {
    let today = Utc::now().date_naive();
    today
        .checked_sub_days(Days::new(1))
        .unwrap_or(today)
}
