use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use ru5ty_gate_database::{Gateway, Venue};
use ru5ty_gate_http::AppError;

#[derive(Debug, Clone)]
pub struct DeviceContext {
    pub gateway: Gateway,
    pub venue: Venue,
}

impl<S: Send + Sync> FromRequestParts<S> for DeviceContext {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
