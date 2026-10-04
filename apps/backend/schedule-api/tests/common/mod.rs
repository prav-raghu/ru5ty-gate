#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::routing::post;
use http_body_util::BodyExt;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_config::EnvReader;
use ru5ty_gate_database::PgPool;
use ru5ty_gate_database::sqlx;
use schedule_api::application::Application;
use schedule_api::config::ServiceConfig;
use serde_json::Value;
use tokio::net::TcpListener;
use tower::ServiceExt;
use uuid::Uuid;

pub const API_KEY: &str = "0123456789abcdef0123456789abcdef-key";

pub fn config_with(extra: &[(&str, &str)]) -> ServiceConfig {
    let mut pairs = vec![
        ("PORT", "4003"),
        ("CORS_ORIGIN", "http://localhost:4000"),
        ("REDIS_URL", "redis://127.0.0.1:6379"),
        ("DATABASE_URL", "postgres://unused"),
        ("SCHEDULE_API_KEY", API_KEY),
    ];
    pairs.extend_from_slice(extra);
    ServiceConfig::from_env(&EnvReader::from_pairs(pairs)).unwrap()
}

pub fn build_application(pool: &PgPool, extra: &[(&str, &str)]) -> Application {
    Application::with_parts(config_with(extra), pool.clone(), RedisService::disabled())
}

#[derive(Clone)]
pub struct Receiver {
    pub status: StatusCode,
    pub received: Arc<Mutex<Vec<(HeaderMap, String)>>>,
}

async fn capture(State(receiver): State<Receiver>, headers: HeaderMap, body: String) -> StatusCode {
    receiver.received.lock().unwrap().push((headers, body));
    receiver.status
}

pub async fn start_receiver(status: StatusCode) -> (String, Receiver) {
    let receiver = Receiver {
        status,
        received: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/hook", post(capture))
        .with_state(receiver.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}/hook"), receiver)
}

pub async fn seed_delivery(pool: &PgPool, url: &str, status: &str, retry_count: i32) -> Uuid {
    let subscription: Uuid = sqlx::query_scalar(
        "INSERT INTO webhook_subscriptions (url, secret, events, retry_count, timeout_seconds) \
         VALUES ($1, $2, ARRAY['user.created'], $3, 5) RETURNING id",
    )
    .bind(url)
    .bind("s".repeat(40))
    .bind(retry_count)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar(
        "INSERT INTO webhook_deliveries (subscription_id, event_type, payload, status) \
         VALUES ($1, 'user.created', '{\"event\":\"user.created\"}', $2) RETURNING id",
    )
    .bind(subscription)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap()
}

pub async fn delivery_state(pool: &PgPool, id: Uuid) -> (String, i32) {
    sqlx::query_as("SELECT status, attempt_count FROM webhook_deliveries WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

pub async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    authorization: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(value) = authorization {
        builder = builder.header("authorization", value);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
