use crate::aggregate::{self, CycleResult};
use crate::clickhouse::ClickhouseHttp;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use deadpool_postgres::Pool;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub ch: ClickhouseHttp,
    pub pg: Pool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/recompute", post(recompute))
        .with_state(Arc::new(state))
}

async fn health() -> &'static str {
    "ok\n"
}

#[derive(Deserialize)]
pub struct RecomputeBody {
    pub date: String,
}

#[derive(serde::Serialize)]
struct RecomputeResponse {
    metric_date: String,
    records_processed: u64,
    duration_ms: u64,
    metrics_written: usize,
}

async fn recompute(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecomputeBody>,
) -> Result<Json<RecomputeResponse>, (StatusCode, String)> {
    let d = parse_date(&body.date).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    tracing::info!(metric_date = %body.date, "aggregation cycle started (manual)");
    let started = std::time::Instant::now();
    let result = run_cycle(&state, d)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    tracing::info!(
        metric_date = %result.metric_date,
        records_processed = result.records_processed,
        duration_ms = result.duration_ms,
        metrics_written = result.metrics_written,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "aggregation cycle finished (manual)"
    );
    Ok(Json(to_response(result)))
}

pub async fn run_cycle(state: &AppState, metric_date: NaiveDate) -> anyhow::Result<CycleResult> {
    aggregate::run_aggregation_cycle(&state.ch, &state.pg, metric_date).await
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    if s.len() != 10 || s.as_bytes()[4] != b'-' || s.as_bytes()[7] != b'-' {
        return Err("expected date format YYYY-MM-DD".into());
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| e.to_string())
}

fn to_response(c: CycleResult) -> RecomputeResponse {
    RecomputeResponse {
        metric_date: c.metric_date.format("%Y-%m-%d").to_string(),
        records_processed: c.records_processed,
        duration_ms: c.duration_ms,
        metrics_written: c.metrics_written,
    }
}
