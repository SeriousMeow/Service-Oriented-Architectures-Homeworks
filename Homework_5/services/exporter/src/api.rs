use crate::export_job::{export_date_to_s3, object_key};
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
    pub pg: Pool,
    pub s3: aws_sdk_s3::Client,
    pub bucket: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/export", post(manual_export))
        .with_state(Arc::new(state))
}

async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct ExportRequest {
    date: String,
}

async fn manual_export(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let d = NaiveDate::parse_from_str(&req.date, "%Y-%m-%d").map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid date (expected YYYY-MM-DD): {e}"),
        )
    })?;
    export_date_to_s3(&state.pg, &state.s3, &state.bucket, d)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "status": "exported",
        "date": req.date,
        "s3_uri": format!("s3://{}/{}", state.bucket, object_key(&req.date)),
    })))
}
