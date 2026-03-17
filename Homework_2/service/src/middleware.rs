use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{error, info};

use crate::auth::AuthContext;

#[derive(Clone)]
pub struct InternalError(pub String);

pub async fn logging(request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let method = request.method().to_string();
    let endpoint = request.uri().path().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let is_mutating = matches!(method.as_str(), "POST" | "PUT" | "DELETE");

    let (parts, body) = request.into_parts();

    let user_id = parts
        .extensions
        .get::<AuthContext>()
        .map(|ctx| ctx.user_id.to_string());

    let (body_log, request) = if is_mutating {
        match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => {
                let text = std::str::from_utf8(&bytes)
                    .unwrap_or("<binary>")
                    .to_string();
                (Some(text), Request::from_parts(parts, Body::from(bytes)))
            }
            Err(_) => (None, Request::from_parts(parts, Body::empty())),
        }
    } else {
        (None, Request::from_parts(parts, body))
    };

    let start = Instant::now();
    let response = next.run(request).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    let status_code = response.status().as_u16();

    let internal_error = response
        .extensions()
        .get::<InternalError>()
        .map(|e| e.0.clone());

    if status_code >= 500 {
        error!(
            request_id = %request_id,
            method = %method,
            endpoint = %endpoint,
            status_code = status_code,
            duration_ms = duration_ms,
            user_id = ?user_id,
            timestamp = %timestamp,
            request_body = ?body_log,
            error = ?internal_error,
            "Internal server error"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            endpoint = %endpoint,
            status_code = status_code,
            duration_ms = duration_ms,
            user_id = ?user_id,
            timestamp = %timestamp,
            request_body = ?body_log,
            "Request completed"
        );
    }

    response
}
