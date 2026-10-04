#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::http::{Method, StatusCode};

use crate::common::{DEAD_UPSTREAM, app_with, call, config, start_upstream};

#[tokio::test]
async fn liveness_never_depends_on_upstreams() {
    let app = app_with(config(DEAD_UPSTREAM, DEAD_UPSTREAM, DEAD_UPSTREAM, &[]));

    let (status, _, body) = call(&app, Method::GET, "/health/live", &[], None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "alive");
    assert_eq!(body["service"], "api-gateway");
}

#[tokio::test]
async fn health_is_degraded_but_not_failing_when_backends_are_down() {
    let app = app_with(config(DEAD_UPSTREAM, DEAD_UPSTREAM, DEAD_UPSTREAM, &[]));

    let (health, _, body) = call(&app, Method::GET, "/health", &[], None).await;
    let (ready, ready_body, _) = call(&app, Method::GET, "/health/ready", &[], None).await;

    assert_eq!(health, StatusCode::OK);
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["checks"]["customer-api"]["status"], "degraded");
    assert_eq!(ready, StatusCode::OK);
    drop(ready_body);
}

#[tokio::test]
async fn aggregated_service_health_maps_to_status_codes() {
    let (healthy, _) = start_upstream("ok", StatusCode::OK).await;

    let all_up = app_with(config(&healthy, &healthy, &healthy, &[]));
    let some_down = app_with(config(&healthy, DEAD_UPSTREAM, &healthy, &[]));
    let all_down = app_with(config(DEAD_UPSTREAM, DEAD_UPSTREAM, DEAD_UPSTREAM, &[]));

    let (up, _, up_body) = call(&all_up, Method::GET, "/health/services", &[], None).await;
    let (partial, _, partial_body) =
        call(&some_down, Method::GET, "/health/services", &[], None).await;
    let (down, _, down_body) = call(&all_down, Method::GET, "/health/services", &[], None).await;

    assert_eq!(up, StatusCode::OK);
    assert_eq!(up_body["status"], "healthy");
    assert_eq!(up_body["services"].as_array().unwrap().len(), 3);
    assert_eq!(partial, StatusCode::MULTI_STATUS);
    assert_eq!(partial_body["status"], "degraded");
    assert_eq!(down, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(down_body["status"], "unhealthy");
    assert!(down_body["services"][0]["error"].is_string());
}

#[tokio::test]
async fn single_service_health_lookups() {
    let (healthy, upstream) = start_upstream("ok", StatusCode::OK).await;
    let (failing, _) = start_upstream("bad", StatusCode::INTERNAL_SERVER_ERROR).await;
    let app = app_with(config(&healthy, &failing, &healthy, &[]));

    let (ok, _, ok_body) = call(
        &app,
        Method::GET,
        "/health/services/customer-api",
        &[],
        None,
    )
    .await;
    let (bad, _, bad_body) = call(&app, Method::GET, "/health/services/admin-api", &[], None).await;
    let (unknown, _, unknown_body) =
        call(&app, Method::GET, "/health/services/other", &[], None).await;

    assert_eq!(ok, StatusCode::OK);
    assert_eq!(ok_body["name"], "customer-api");
    assert_eq!(
        upstream.seen.lock().unwrap()[0].path_and_query,
        "/api/v1/ping"
    );
    assert_eq!(bad, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(bad_body["error"], "HTTP 500");
    assert_eq!(unknown, StatusCode::NOT_FOUND);
    assert_eq!(unknown_body["error"], "Service not found");
}
