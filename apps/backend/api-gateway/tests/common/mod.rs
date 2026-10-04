#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use api_gateway::application::Application;
use api_gateway::config::ServiceConfig;
use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use http_body_util::BodyExt;
use ru5ty_gate_config::EnvReader;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tower::ServiceExt;

#[derive(Debug, Clone)]
pub struct Seen {
    pub method: Method,
    pub path_and_query: String,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

#[derive(Clone)]
pub struct Upstream {
    pub name: &'static str,
    pub status: StatusCode,
    pub seen: Arc<Mutex<Vec<Seen>>>,
}

async fn record(
    axum::extract::State(upstream): axum::extract::State<Upstream>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();
    upstream.seen.lock().unwrap().push(Seen {
        method: parts.method.clone(),
        path_and_query: parts
            .uri
            .path_and_query()
            .map(ToString::to_string)
            .unwrap_or_default(),
        headers: parts.headers.clone(),
        body: bytes,
    });
    let mut response = (
        upstream.status,
        axum::Json(json!({ "from": upstream.name, "path": parts.uri.path() })),
    )
        .into_response();
    response
        .headers_mut()
        .insert("x-upstream", upstream.name.parse().unwrap());
    response
        .headers_mut()
        .insert("connection", "keep-alive".parse().unwrap());
    response
}

pub async fn start_upstream(name: &'static str, status: StatusCode) -> (String, Upstream) {
    let upstream = Upstream {
        name,
        status,
        seen: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .fallback(any(record))
        .with_state(upstream.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), upstream)
}

pub const DEAD_UPSTREAM: &str = "http://127.0.0.1:1";

pub fn config(
    customer: &str,
    admin: &str,
    scheduler: &str,
    extra: &[(&str, &str)],
) -> ServiceConfig {
    let mut pairs = vec![
        ("PORT", "4000"),
        ("CORS_ORIGIN", "http://localhost:3000"),
        ("CUSTOMER_API_URL", customer),
        ("ADMIN_API_URL", admin),
        ("SCHEDULER_API_URL", scheduler),
    ];
    pairs.extend_from_slice(extra);
    ServiceConfig::from_env(&EnvReader::from_pairs(pairs)).unwrap()
}

pub async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let request = builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let response_headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        response_headers,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

pub fn app_with(config: ServiceConfig) -> Router {
    Application::with_parts(config, None).router()
}

pub async fn call_from_peer(
    app: &Router,
    peer: &str,
    uri: &str,
    headers: &[(&str, &str)],
) -> StatusCode {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        peer.parse::<std::net::SocketAddr>().unwrap(),
    ));
    app.clone().oneshot(request).await.unwrap().status()
}
