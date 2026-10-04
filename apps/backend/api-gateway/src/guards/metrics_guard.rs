use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::AppError;
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct MetricsToken(Arc<str>);

impl MetricsToken {
    pub fn new(token: &str) -> Self {
        Self(Arc::from(token))
    }

    pub fn matches(&self, candidate: &str) -> bool {
        let expected = self.0.as_bytes();
        let provided = candidate.as_bytes();
        expected.len() == provided.len() && bool::from(expected.ct_eq(provided))
    }
}

pub async fn metrics_guard(
    State(token): State<MetricsToken>,
    request: Request,
    next: Next,
) -> Response {
    let provided = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim);
    match provided {
        Some(candidate) if token.matches(candidate) => next.run(request).await,
        _ => AppError::Unauthorized.into_response(),
    }
}
