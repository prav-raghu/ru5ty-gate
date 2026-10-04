#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../common/mod.rs"]
mod common;

use axum::http::{Method, StatusCode};
use ru5ty_gate_database::sqlx::{self, PgPool};

use crate::common::{
    API_KEY, build_application, call, delivery_state, seed_delivery, start_receiver,
};

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn public_endpoints_respond_and_carry_timestamps(pool: PgPool) {
    let app = build_application(&pool, &[]).router();

    let (ping, ping_body) = call(&app, Method::GET, "/api/v1/ping", None).await;
    let (ready, ready_body) = call(&app, Method::GET, "/api/v1/ready", None).await;
    let (v2, v2_body) = call(&app, Method::GET, "/api/v2/health", None).await;
    let (missing, _) = call(&app, Method::GET, "/api/v1/nope", None).await;

    assert_eq!(ping, StatusCode::OK);
    assert_eq!(ping_body["status"], "pong");
    assert!(ping_body["responseDateTime"].is_string());
    assert_eq!(ready, StatusCode::OK);
    assert_eq!(ready_body["db"], "ok");
    assert_eq!(v2, StatusCode::OK);
    assert_eq!(v2_body["version"], "v2");
    assert_eq!(missing, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn job_routes_require_the_api_key(pool: PgPool) {
    let app = build_application(&pool, &[]).router();
    let valid = format!("Api-Key {API_KEY}");
    let wrong = "Api-Key 0123456789abcdef0123456789abcdef-nop";

    let (anonymous, _) = call(&app, Method::GET, "/api/v1/jobs", None).await;
    let (bearer, _) = call(
        &app,
        Method::GET,
        "/api/v1/jobs",
        Some(&format!("Bearer {API_KEY}")),
    )
    .await;
    let (bad_key, _) = call(&app, Method::GET, "/api/v1/jobs", Some(wrong)).await;
    let (short, _) = call(&app, Method::GET, "/api/v1/jobs", Some("Api-Key short")).await;
    let (ok, body) = call(&app, Method::GET, "/api/v1/jobs", Some(&valid)).await;

    for status in [anonymous, bearer, bad_key, short] {
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(ok, StatusCode::OK);
    assert_eq!(body["data"][0]["name"], "webhookProcessor");
    assert_eq!(body["data"][0]["intervalSeconds"], 60);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn running_the_webhook_job_on_demand_delivers_pending_webhooks(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::OK).await;
    let id = seed_delivery(&pool, &url, "pending", 3).await;
    let app = build_application(&pool, &[]).router();
    let valid = format!("Api-Key {API_KEY}");

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/jobs/webhookProcessor/run",
        Some(&valid),
    )
    .await;
    let (unknown, _) = call(&app, Method::POST, "/api/v1/jobs/nothing/run", Some(&valid)).await;
    let (unauthorised, _) = call(
        &app,
        Method::POST,
        "/api/v1/jobs/webhookProcessor/run",
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "Job executed");
    assert_eq!(receiver.received.lock().unwrap().len(), 1);
    assert_eq!(delivery_state(&pool, id).await, ("delivered".to_owned(), 1));
    assert_eq!(unknown, StatusCode::NOT_FOUND);
    assert_eq!(unauthorised, StatusCode::UNAUTHORIZED);
}
