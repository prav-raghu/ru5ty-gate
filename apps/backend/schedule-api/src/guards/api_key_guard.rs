use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::AppError;
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct ApiKey(Arc<str>);

impl ApiKey {
    pub fn new(key: &str) -> Self {
        Self(Arc::from(key))
    }

    pub fn matches(&self, candidate: &str) -> bool {
        let expected = self.0.as_bytes();
        let provided = candidate.as_bytes();
        expected.len() == provided.len() && bool::from(expected.ct_eq(provided))
    }
}

pub async fn api_key_guard(State(key): State<ApiKey>, request: Request, next: Next) -> Response {
    let provided = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Api-Key "))
        .map(str::trim);
    match provided {
        Some(candidate) if key.matches(candidate) => next.run(request).await,
        _ => AppError::Unauthorized.into_response(),
    }
}
