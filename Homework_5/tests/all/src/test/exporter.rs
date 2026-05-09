use anyhow::{bail, Context, Result};
use aws_sdk_s3::Client as S3Client;
use serde::Deserialize;

use crate::test::common::Config;

#[derive(Debug, Deserialize)]
struct ExportResponse {
    status: String,
    date: String,
    s3_uri: String,
}

pub async fn export_and_verify(
    client: &reqwest::Client,
    s3: &S3Client,
    cfg: &Config,
    metric_date: &str,
) -> Result<String> {
    let export_url = format!("{}/export", cfg.exporter_url.trim_end_matches('/'));
    let export_resp = client
        .post(&export_url)
        .json(&serde_json::json!({ "date": metric_date }))
        .send()
        .await
        .with_context(|| format!("POST {}", export_url))?;
    if !export_resp.status().is_success() {
        bail!(
            "exporter /export failed: {}",
            export_resp.text().await.unwrap_or_default()
        );
    }
    let export_body: ExportResponse = export_resp.json().await.context("parse export response")?;
    let expected_key = format!("daily/{}/aggregates.json", metric_date);
    if !export_body.s3_uri.ends_with(&expected_key) || export_body.status != "exported" {
        bail!("unexpected export response payload: {:?}", export_body);
    }
    if export_body.date != metric_date {
        bail!(
            "export response date mismatch: expected {}, got {}",
            metric_date,
            export_body.date
        );
    }

    s3.head_object()
        .bucket(&cfg.minio_bucket)
        .key(&expected_key)
        .send()
        .await
        .with_context(|| format!("head_object s3://{}/{}", cfg.minio_bucket, expected_key))?;

    Ok(expected_key)
}
