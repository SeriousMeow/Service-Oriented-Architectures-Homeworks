use crate::metrics::Metrics;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Router, extract::State, response::IntoResponse};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct AppState {
    pub metrics: Arc<Metrics>,
    pub kafka_healthy: Arc<AtomicBool>,
    pub cassandra_healthy: Arc<AtomicBool>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let kafka_ok = state.kafka_healthy.load(Ordering::Relaxed);
    let cassandra_ok = state.cassandra_healthy.load(Ordering::Relaxed);
    if kafka_ok && cassandra_ok {
        (StatusCode::OK, "ok").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "unhealthy").into_response()
    }
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    match state.metrics.render() {
        Ok(body) => (
            StatusCode::OK,
            [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
            body,
        )
            .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}
