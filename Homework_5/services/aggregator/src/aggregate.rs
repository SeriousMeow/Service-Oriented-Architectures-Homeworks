use crate::clickhouse::{AggRow, ClickhouseHttp};
use crate::pg;
use anyhow::Context;
use chrono::{Days, NaiveDate, Utc};
use deadpool_postgres::Pool;
use serde_json::json;

pub struct CycleResult {
    pub metric_date: NaiveDate,
    pub records_processed: u64,
    pub duration_ms: u64,
    pub metrics_written: usize,
}

pub async fn run_aggregation_cycle(
    ch: &ClickhouseHttp,
    pg_pool: &Pool,
    metric_date: NaiveDate,
) -> anyhow::Result<CycleResult> {
    let start = std::time::Instant::now();
    let date_str = metric_date.format("%Y-%m-%d").to_string();

    let records_processed = ch.count_events_for_day(&date_str).await?;

    let dau = ch.metric_dau(&date_str).await?;
    let avg_watch = ch.metric_avg_watch(&date_str).await?;
    let conversion = ch.metric_conversion(&date_str).await?;
    let (ret_d1, ret_d7) = ch.metric_retention(&date_str).await?;
    let top = ch.top_movies(&date_str, 10).await?;
    let top_json = json!(top
        .into_iter()
        .map(|(movie_id, c)| json!({ "movie_id": movie_id, "views": c }))
        .collect::<Vec<_>>());

    let computed_at = Utc::now();
    let ch_ts = computed_at.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    let agg_rows = vec![
        AggRow {
            metric_date: date_str.clone(),
            metric_name: "dau".into(),
            metric_value: dau,
            metric_payload: String::new(),
            computed_at: ch_ts.clone(),
        },
        AggRow {
            metric_date: date_str.clone(),
            metric_name: "avg_watch_seconds".into(),
            metric_value: avg_watch,
            metric_payload: String::new(),
            computed_at: ch_ts.clone(),
        },
        AggRow {
            metric_date: date_str.clone(),
            metric_name: "conversion_rate".into(),
            metric_value: conversion,
            metric_payload: String::new(),
            computed_at: ch_ts.clone(),
        },
        AggRow {
            metric_date: date_str.clone(),
            metric_name: "retention_d1".into(),
            metric_value: ret_d1,
            metric_payload: String::new(),
            computed_at: ch_ts.clone(),
        },
        AggRow {
            metric_date: date_str.clone(),
            metric_name: "retention_d7".into(),
            metric_value: ret_d7,
            metric_payload: String::new(),
            computed_at: ch_ts.clone(),
        },
        AggRow {
            metric_date: date_str.clone(),
            metric_name: "top_movies".into(),
            metric_value: 0.0,
            metric_payload: top_json.to_string(),
            computed_at: ch_ts,
        },
    ];

    ch.insert_agg_rows(&agg_rows)
        .await
        .context("clickhouse insert agg_daily_metrics")?;

    let mut metrics_written = 0usize;
    pg::upsert_metric(
        pg_pool,
        &date_str,
        "dau",
        Some(dau),
        None,
        computed_at,
    )
    .await?;
    metrics_written += 1;
    pg::upsert_metric(
        pg_pool,
        &date_str,
        "avg_watch_seconds",
        Some(avg_watch),
        None,
        computed_at,
    )
    .await?;
    metrics_written += 1;
    pg::upsert_metric(
        pg_pool,
        &date_str,
        "conversion_rate",
        Some(conversion),
        None,
        computed_at,
    )
    .await?;
    metrics_written += 1;
    pg::upsert_metric(
        pg_pool,
        &date_str,
        "retention_d1",
        Some(ret_d1),
        None,
        computed_at,
    )
    .await?;
    metrics_written += 1;
    pg::upsert_metric(
        pg_pool,
        &date_str,
        "retention_d7",
        Some(ret_d7),
        None,
        computed_at,
    )
    .await?;
    metrics_written += 1;
    pg::upsert_metric(
        pg_pool,
        &date_str,
        "top_movies",
        None,
        Some(top_json),
        computed_at,
    )
    .await?;
    metrics_written += 1;

    let duration_ms = start.elapsed().as_millis() as u64;
    Ok(CycleResult {
        metric_date,
        records_processed,
        duration_ms,
        metrics_written,
    })
}

pub fn yesterday_utc() -> NaiveDate {
    let today = Utc::now().date_naive();
    today
        .checked_sub_days(Days::new(1))
        .unwrap_or(today)
}
