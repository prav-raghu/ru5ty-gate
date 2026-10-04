#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::common::{DEAD_UPSTREAM, app_with, call, call_from_peer, config, start_upstream};

#[tokio::test]
async fn customer_traffic_keeps_its_path_and_query() {
    let (customer, customer_seen) = start_upstream("customer", StatusCode::OK).await;
    let (admin, admin_seen) = start_upstream("admin", StatusCode::OK).await;
    let (scheduler, _) = start_upstream("scheduler", StatusCode::OK).await;
    let app = app_with(config(&customer, &admin, &scheduler, &[]));

    let (status, headers, body) = call(
        &app,
        Method::GET,
        "/api/v1/users?limit=5&gender=female",
        &[("authorization", "Bearer abc")],
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["from"], "customer");
    assert_eq!(headers["x-upstream"], "customer");
    let seen = customer_seen.seen.lock().unwrap();
    assert_eq!(
        seen[0].path_and_query,
        "/api/v1/users?limit=5&gender=female"
    );
    assert_eq!(seen[0].headers["authorization"], "Bearer abc");
    assert!(admin_seen.seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn admin_and_scheduler_prefixes_are_stripped() {
    let (customer, _) = start_upstream("customer", StatusCode::OK).await;
    let (admin, admin_seen) = start_upstream("admin", StatusCode::OK).await;
    let (scheduler, scheduler_seen) = start_upstream("scheduler", StatusCode::OK).await;
    let app = app_with(config(&customer, &admin, &scheduler, &[]));

    let (_, _, admin_body) = call(&app, Method::GET, "/admin/api/v1/auth/me?x=1", &[], None).await;
    let (_, _, scheduler_body) = call(&app, Method::GET, "/scheduler/api/v1/ping", &[], None).await;
    let (_, _, bare) = call(&app, Method::GET, "/admin", &[], None).await;

    assert_eq!(admin_body["from"], "admin");
    assert_eq!(
        admin_seen.seen.lock().unwrap()[0].path_and_query,
        "/api/v1/auth/me?x=1"
    );
    assert_eq!(scheduler_body["from"], "scheduler");
    assert_eq!(
        scheduler_seen.seen.lock().unwrap()[0].path_and_query,
        "/api/v1/ping"
    );
    assert_eq!(bare["path"], "/");
}

#[tokio::test]
async fn bodies_methods_and_upstream_statuses_are_forwarded() {
    let (customer, customer_seen) = start_upstream("customer", StatusCode::CREATED).await;
    let app = app_with(config(&customer, &customer, &customer, &[]));
    let payload = json!({ "username": "Pat Smith", "nested": { "a": [1, 2, 3] } });

    let (status, _, _) = call(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        &[],
        Some(payload.clone()),
    )
    .await;
    let (put_status, _, _) = call(
        &app,
        Method::PUT,
        "/api/v1/webhooks/subscriptions/1",
        &[],
        Some(payload.clone()),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(put_status, StatusCode::CREATED);
    let seen = customer_seen.seen.lock().unwrap();
    assert_eq!(seen[0].method, Method::POST);
    assert_eq!(seen[1].method, Method::PUT);
    let forwarded: serde_json::Value = serde_json::from_slice(&seen[0].body).unwrap();
    assert_eq!(forwarded, payload);
}

#[tokio::test]
async fn forwarding_headers_are_added_and_hop_by_hop_headers_removed() {
    let (customer, customer_seen) = start_upstream("customer", StatusCode::OK).await;
    let app = app_with(config(&customer, &customer, &customer, &[]));

    let (_, response_headers, _) = call(
        &app,
        Method::GET,
        "/api/v1/ping",
        &[
            ("x-forwarded-for", "203.0.113.9"),
            ("host", "gateway.example.com"),
            ("te", "trailers"),
            ("x-custom", "kept"),
        ],
        None,
    )
    .await;

    let seen = customer_seen.seen.lock().unwrap();
    let headers = &seen[0].headers;
    assert!(
        headers["x-forwarded-for"]
            .to_str()
            .unwrap()
            .starts_with("203.0.113.9")
    );
    assert_eq!(headers["x-forwarded-host"], "gateway.example.com");
    assert_eq!(headers["x-custom"], "kept");
    assert!(headers.get("te").is_none());
    assert!(response_headers.get("connection").is_none());
}

#[tokio::test]
async fn unreachable_upstreams_return_502_with_the_envelope() {
    let app = app_with(config(DEAD_UPSTREAM, DEAD_UPSTREAM, DEAD_UPSTREAM, &[]));

    let (status, _, body) = call(&app, Method::GET, "/api/v1/ping", &[], None).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body["isSuccessful"], false);
    assert_eq!(body["message"], "Bad gateway");
}

#[tokio::test]
async fn unknown_paths_return_the_not_found_envelope_and_security_headers() {
    let (customer, _) = start_upstream("customer", StatusCode::OK).await;
    let app = app_with(config(&customer, &customer, &customer, &[]));

    let (status, headers, body) = call(&app, Method::GET, "/nothing/here", &[], None).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "Not found");
    assert_eq!(headers["x-content-type-options"], "nosniff");
}

#[tokio::test]
async fn the_gateway_rate_limit_applies_before_proxying() {
    let (customer, customer_seen) = start_upstream("customer", StatusCode::OK).await;
    let app = app_with(config(
        &customer,
        &customer,
        &customer,
        &[("RATE_LIMIT_MAX", "3")],
    ));
    let mut last = StatusCode::OK;

    for _ in 0..4 {
        last = call(&app, Method::GET, "/api/v1/ping", &[], None).await.0;
    }

    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(customer_seen.seen.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn cors_preflights_are_answered_for_the_configured_origin() {
    let (customer, _) = start_upstream("customer", StatusCode::OK).await;
    let app = app_with(config(&customer, &customer, &customer, &[]));

    let (status, headers, _) = call(
        &app,
        Method::OPTIONS,
        "/api/v1/auth/login",
        &[
            ("origin", "http://localhost:3000"),
            ("access-control-request-method", "POST"),
        ],
        None,
    )
    .await;

    assert!(status.is_success());
    assert_eq!(
        headers["access-control-allow-origin"],
        "http://localhost:3000"
    );
    assert_eq!(headers["access-control-allow-credentials"], "true");
}

#[tokio::test]
async fn the_upstream_receives_only_the_client_resolved_behind_the_trusted_proxy() {
    let (customer, customer_seen) = start_upstream("customer", StatusCode::OK).await;
    let app = app_with(config(&customer, &customer, &customer, &[]));

    let status = call_from_peer(
        &app,
        "10.0.0.2:4000",
        "/api/v1/ping",
        &[("x-forwarded-for", "6.6.6.6, 198.51.100.7")],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let seen = customer_seen.seen.lock().unwrap();
    assert_eq!(seen[0].headers["x-forwarded-for"], "198.51.100.7");
}
