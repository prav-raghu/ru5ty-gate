use axum::Json;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{CONNECTION, CONTENT_LENGTH, HOST, TRANSFER_ENCODING, UPGRADE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::ClientIp;
use serde_json::json;

use crate::config::ProxyTarget;

const HOP_BY_HOP: [&str; 8] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

fn is_hop_by_hop(name: &HeaderName) -> bool {
    HOP_BY_HOP.contains(&name.as_str())
        || name == CONNECTION
        || name == TRANSFER_ENCODING
        || name == UPGRADE
}

fn forwarded_headers(original: &HeaderMap, client_ip: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in original {
        if is_hop_by_hop(name) || name == HOST || name == CONTENT_LENGTH {
            continue;
        }
        headers.append(name.clone(), value.clone());
    }
    if let Ok(value) = HeaderValue::from_str(client_ip) {
        headers.insert("x-forwarded-for", value);
    }
    if let Some(host) = original.get(HOST) {
        headers.insert("x-forwarded-host", host.clone());
    }
    headers
}

pub fn upstream_url(target: &ProxyTarget, path: &str, query: Option<&str>) -> String {
    let forwarded_path = if target.strip_prefix {
        let remainder = path.strip_prefix(target.prefix).unwrap_or(path);
        if remainder.is_empty() { "/" } else { remainder }
    } else {
        path
    };
    match query {
        Some(query) => format!("{}{forwarded_path}?{query}", target.upstream),
        None => format!("{}{forwarded_path}", target.upstream),
    }
}

fn gateway_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({ "isSuccessful": false, "message": message })),
    )
        .into_response()
}

pub struct ProxyController;

impl ProxyController {
    pub async fn forward(
        State(target): State<ProxyTarget>,
        ClientIp(client_ip): ClientIp,
        request: Request,
    ) -> Response {
        let (parts, body) = request.into_parts();
        let url = upstream_url(&target, parts.uri.path(), parts.uri.query());
        let outbound = target
            .client
            .request(parts.method.clone(), &url)
            .headers(forwarded_headers(&parts.headers, &client_ip))
            .body(reqwest::Body::wrap_stream(body.into_data_stream()));

        let upstream = match outbound.send().await {
            Ok(response) => response,
            Err(error) if error.is_timeout() => {
                tracing::error!(service = target.name, %error, "upstream timed out");
                return gateway_error(StatusCode::GATEWAY_TIMEOUT, "Gateway timeout");
            }
            Err(error) => {
                tracing::error!(service = target.name, %error, "upstream unavailable");
                return gateway_error(StatusCode::BAD_GATEWAY, "Bad gateway");
            }
        };

        let mut response = Response::builder().status(upstream.status());
        if let Some(headers) = response.headers_mut() {
            for (name, value) in upstream.headers() {
                if !is_hop_by_hop(name) {
                    headers.append(name.clone(), value.clone());
                }
            }
        }
        response
            .body(Body::from_stream(upstream.bytes_stream()))
            .unwrap_or_else(|_| gateway_error(StatusCode::BAD_GATEWAY, "Bad gateway"))
    }
}
