use crate::event::{MovieEventPayload, ValidationError};
use crate::publisher::Publisher;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub publisher: Arc<Publisher>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/events", post(post_event))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn post_event(
    State(state): State<AppState>,
    Json(payload): Json<MovieEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    payload
        .validate()
        .map_err(|e: ValidationError| ApiError::bad_request(e.to_string()))?;
    state
        .publisher
        .publish(&payload)
        .await
        .map_err(|e| ApiError::internal(e))?;
    Ok(Json(json!({ "event_id": payload.event_id })))
}

pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }

    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}
