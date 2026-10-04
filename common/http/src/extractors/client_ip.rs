use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use ru5ty_gate_logging::normalize_ip;

use crate::extractors::TrustedProxyHops;

const UNKNOWN: &str = "unknown";

#[derive(Debug, Clone)]
pub struct ClientIp(pub String);

impl ClientIp {
    pub fn resolve(parts: &Parts) -> String {
        let hops = parts
            .extensions
            .get::<TrustedProxyHops>()
            .map_or(0, |hops| hops.0);

        let mut chain: Vec<String> = parts
            .headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| normalize_ip(value).to_owned())
            .collect();
        if let Some(ConnectInfo(address)) = parts.extensions.get::<ConnectInfo<SocketAddr>>() {
            chain.push(normalize_ip(&address.ip().to_string()).to_owned());
        }

        let index = chain.len().saturating_sub(1 + hops);
        match chain.get(index) {
            Some(candidate) if candidate.parse::<IpAddr>().is_ok() => candidate.clone(),
            _ => UNKNOWN.to_owned(),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(Self::resolve(parts)))
    }
}
