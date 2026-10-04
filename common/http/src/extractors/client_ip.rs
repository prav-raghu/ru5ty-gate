use std::net::SocketAddr;

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use ru5ty_gate_logging::normalize_ip;

#[derive(Debug, Clone)]
pub struct ClientIp(pub String);

impl ClientIp {
    pub fn resolve(parts: &Parts) -> String {
        let forwarded = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(address) = forwarded {
            return normalize_ip(address).to_owned();
        }
        parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map_or_else(
                || "unknown".to_owned(),
                |ConnectInfo(address)| normalize_ip(&address.ip().to_string()).to_owned(),
            )
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(Self::resolve(parts)))
    }
}
