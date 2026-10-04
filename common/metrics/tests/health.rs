#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use ru5ty_gate_metrics::{HealthCheckBuilder, HealthState, health_router};
use serde_json::Value;
use tower::ServiceExt;

async fn get_json(path: &str, builder: HealthCheckBuilder) -> (StatusCode, Value) {
    let state = HealthState::new("test-service", "1.0.0", builder.build());
    let response = health_router(state)
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn healthy_checks_return_200() {
    let builder = HealthCheckBuilder::new()
        .add_database_check(|| async { true })
        .add_redis_check(|| async { true });

    let (status, body) = get_json("/health", builder).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "test-service");
    assert_eq!(body["checks"]["database"]["status"], "healthy");
}

#[tokio::test]
async fn failing_critical_check_returns_503() {
    let builder = HealthCheckBuilder::new().add_database_check(|| async { false });

    let (status, body) = get_json("/health", builder).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["status"], "unhealthy");
}

#[tokio::test]
async fn failing_non_critical_check_degrades_but_stays_ready() {
    let builder = HealthCheckBuilder::new().add_redis_check(|| async { false });

    let (health_status, health_body) = get_json("/health", builder).await;
    assert_eq!(health_status, StatusCode::OK);
    assert_eq!(health_body["status"], "degraded");

    let builder = HealthCheckBuilder::new().add_redis_check(|| async { false });
    let (ready_status, ready_body) = get_json("/health/ready", builder).await;
    assert_eq!(ready_status, StatusCode::OK);
    assert_eq!(ready_body["status"], "ready");
}

#[tokio::test]
async fn liveness_never_runs_checks() {
    let builder = HealthCheckBuilder::new().add_database_check(|| async { false });

    let (status, body) = get_json("/health/live", builder).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "alive");
}
