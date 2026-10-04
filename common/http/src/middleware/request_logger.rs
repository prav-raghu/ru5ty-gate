use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use ru5ty_gate_logging::mask_sensitive;
use tracing::Level;

const MAX_LOGGED_BODY_BYTES: usize = 1024 * 1024;
const SKIPPED_PATHS: [&str; 2] = ["/health", "/ready"];

fn should_skip(path: &str) -> bool {
    path.starts_with("/docs")
        || SKIPPED_PATHS
            .iter()
            .any(|skipped| path == *skipped || path.ends_with(skipped))
}

pub async fn request_logger(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    if request.method() == Method::OPTIONS || should_skip(&path) {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let correlation_id = request
        .headers()
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    tracing::info!(%method, url = %request.uri(), correlation_id = correlation_id.as_deref(), "Incoming request");

    let request = if method != Method::GET && tracing::enabled!(Level::DEBUG) {
        let (parts, body) = request.into_parts();
        match to_bytes(body, MAX_LOGGED_BODY_BYTES).await {
            Ok(bytes) => {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    tracing::debug!(body = %mask_sensitive(&value), "Request body");
                }
                Request::from_parts(parts, Body::from(bytes))
            }
            Err(_) => Request::from_parts(parts, Body::empty()),
        }
    } else {
        request
    };

    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    let elapsed_ms = started.elapsed().as_millis();
    if status >= 500 {
        tracing::error!(%method, %path, status, elapsed_ms, "Request failed");
    } else {
        tracing::info!(%method, %path, status, elapsed_ms, "Request completed");
    }
    response
}
