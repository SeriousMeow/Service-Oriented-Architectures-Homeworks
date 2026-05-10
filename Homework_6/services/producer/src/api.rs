use crate::config::Config;
use crate::generator;
use crate::publisher::Publisher;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use warehouse_common::{EventType, ValidationError, WarehouseEventPayload};

#[derive(Clone)]
pub struct AppState {
    pub publisher: Arc<Publisher>,
    pub config: Config,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/events", post(post_event))
        .route("/events/product-received", post(post_product_received))
        .route("/events/product-shipped", post(post_product_shipped))
        .route("/events/product-moved", post(post_product_moved))
        .route("/events/product-reserved", post(post_product_reserved))
        .route("/events/product-released", post(post_product_released))
        .route("/events/inventory-counted", post(post_inventory_counted))
        .route("/events/order-created", post(post_order_created))
        .route("/events/order-completed", post(post_order_completed))
        .route("/load/generate", post(post_load_generate))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn post_event(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_payload(&state.publisher, payload).await
}

async fn post_product_received(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::ProductReceived, payload).await
}

async fn post_product_shipped(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::ProductShipped, payload).await
}

async fn post_product_moved(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::ProductMoved, payload).await
}

async fn post_product_reserved(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::ProductReserved, payload).await
}

async fn post_product_released(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::ProductReleased, payload).await
}

async fn post_inventory_counted(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::InventoryCounted, payload).await
}

async fn post_order_created(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::OrderCreated, payload).await
}

async fn post_order_completed(
    State(state): State<AppState>,
    Json(payload): Json<WarehouseEventPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    publish_typed_payload(&state.publisher, EventType::OrderCompleted, payload).await
}

#[derive(Deserialize)]
struct LoadRequest {
    count: Option<usize>,
}

async fn post_load_generate(
    State(state): State<AppState>,
    Json(request): Json<LoadRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = request.count.unwrap_or(state.config.load_default_count);
    let (ok, failed) = generator::generate_batch(&state.publisher, count)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({
        "requested": count,
        "published": ok,
        "failed": failed
    })))
}

async fn publish_typed_payload(
    publisher: &Arc<Publisher>,
    event_type: EventType,
    mut payload: WarehouseEventPayload,
) -> Result<Json<serde_json::Value>, ApiError> {
    payload.event_type = event_type;
    publish_payload(publisher, payload).await
}

async fn publish_payload(
    publisher: &Arc<Publisher>,
    payload: WarehouseEventPayload,
) -> Result<Json<serde_json::Value>, ApiError> {
    payload
        .validate()
        .map_err(|e: ValidationError| ApiError::bad_request(e.to_string()))?;
    publisher
        .publish(&payload)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({
        "event_id": payload.event_id,
        "event_type": payload.event_type.as_str()
    })))
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
