#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../common/mod.rs"]
mod common;

use axum::http::{Method, StatusCode};
use ru5ty_gate_database::sqlx::{self, PgPool};
use serde_json::json;

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, call, live_redis,
    login_token, unique_name,
};

async fn router(pool: &PgPool) -> axum::Router {
    build_application(pool, RecordingEmailSender::new(true), live_redis().await)
        .await
        .router()
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn public_endpoints_respond_with_expected_shapes_and_headers(pool: PgPool) {
    let app = router(&pool).await;

    let (ping_status, ping) = call(&app, Method::GET, "/api/v1/ping", None, None).await;
    let (ready_status, ready) = call(&app, Method::GET, "/api/v1/ready", None, None).await;
    let (v2_status, v2) = call(&app, Method::GET, "/api/v2/health", None, None).await;
    let (missing_status, missing) = call(&app, Method::GET, "/api/v1/nope", None, None).await;

    assert_eq!(ping_status, StatusCode::OK);
    assert_eq!(ping["status"], "pong");
    assert_eq!(ready_status, StatusCode::OK);
    assert_eq!(ready["db"], "ok");
    assert_eq!(v2_status, StatusCode::OK);
    assert_eq!(v2["version"], "v2");
    assert_eq!(missing_status, StatusCode::NOT_FOUND);
    assert_eq!(missing["isSuccessful"], false);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn responses_carry_security_and_version_headers(pool: PgPool) {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let app = router(&pool).await;

    let response = app
        .oneshot(Request::get("/api/v1/ping").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["x-api-version"], "v1");
    assert_eq!(response.headers()["deprecation"], "true");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn validation_errors_use_the_field_level_envelope(pool: PgPool) {
    let app = router(&pool).await;

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        Some(&json!({
            "username": "ab",
            "password": "short",
            "email": "not-an-email",
            "age": 5,
            "genderId": "x",
            "acceptTermsAndConditions": true,
            "unexpected": true
        })),
        None,
    )
    .await;
    let (_, missing) = call(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&json!({ "username": "valid name" })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "Validation failed");
    assert_eq!(body["isSuccessful"], false);
    assert!(
        body["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["field"] == "unexpected")
    );
    assert_eq!(missing["errors"][0]["field"], "password");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn protected_routes_reject_missing_and_invalid_tokens(pool: PgPool) {
    let app = router(&pool).await;

    let (anonymous, _) = call(&app, Method::GET, "/api/v1/users", None, None).await;
    let (invalid, body) = call(&app, Method::GET, "/api/v1/users", None, Some("garbage")).await;
    let (webhooks, _) = call(
        &app,
        Method::GET,
        "/api/v1/webhooks/subscriptions",
        None,
        None,
    )
    .await;

    assert_eq!(anonymous, StatusCode::UNAUTHORIZED);
    assert_eq!(invalid, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "Unauthorized");
    assert_eq!(webhooks, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn full_registration_to_user_listing_flow(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email.clone(), live_redis().await)
        .await
        .router();
    UserFactory::new(&pool, "Existing Friend").create().await;

    let (created, body) = call(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        Some(&json!({
            "username": "Brand New",
            "password": TEST_PASSWORD,
            "email": "brandnew@example.com",
            "age": 28,
            "genderId": "female",
            "acceptTermsAndConditions": true
        })),
        None,
    )
    .await;
    assert_eq!(created, StatusCode::CREATED, "{body}");
    assert!(body["dateTimeStamp"].is_string());

    let (blocked, blocked_body) = call(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&json!({ "username": "Brand New", "password": TEST_PASSWORD })),
        None,
    )
    .await;
    assert_eq!(blocked, StatusCode::UNAUTHORIZED);
    assert_eq!(
        blocked_body["message"],
        "Please verify your email before logging in"
    );

    let link = email.last().unwrap().variables["verificationLink"].clone();
    let token = link.rsplit("token=").next().unwrap();
    let (verified, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/auth/verify/{token}"),
        None,
        None,
    )
    .await;
    assert_eq!(verified, StatusCode::OK);

    let access = login_token(&app, "Brand New").await;
    let (listed, list) = call(
        &app,
        Method::GET,
        "/api/v1/users?limit=10",
        None,
        Some(&access),
    )
    .await;
    assert_eq!(listed, StatusCode::OK);
    let names: Vec<_> = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|user| user["username"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Existing Friend"]);
    assert!(list["data"][0]["lastSeen"].is_string());

    let id = list["data"][0]["id"].as_str().unwrap();
    let (single, user) = call(
        &app,
        Method::GET,
        &format!("/api/v1/users/{id}"),
        None,
        Some(&access),
    )
    .await;
    let (bad_id, _) = call(
        &app,
        Method::GET,
        "/api/v1/users/not-a-uuid",
        None,
        Some(&access),
    )
    .await;
    let (unknown, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/users/{}", uuid::Uuid::new_v4()),
        None,
        Some(&access),
    )
    .await;
    assert_eq!(single, StatusCode::OK);
    assert_eq!(user["data"]["username"], "Existing Friend");
    assert_eq!(bad_id, StatusCode::BAD_REQUEST);
    assert_eq!(unknown, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn customers_cannot_dump_the_user_export(pool: PgPool) {
    let app = router(&pool).await;
    UserFactory::new(&pool, "Curious Customer").create().await;
    let access = login_token(&app, "Curious Customer").await;

    let (buffered, _) = call(
        &app,
        Method::GET,
        "/api/v1/users/export",
        None,
        Some(&access),
    )
    .await;
    let (streamed, _) = call(
        &app,
        Method::GET,
        "/api/v1/users/export/stream",
        None,
        Some(&access),
    )
    .await;

    assert_eq!(buffered, StatusCode::FORBIDDEN);
    assert_eq!(streamed, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn webhook_crud_over_http(pool: PgPool) {
    let app = router(&pool).await;
    UserFactory::new(&pool, "Webhook Fan").create().await;
    let access = login_token(&app, "Webhook Fan").await;

    let (invalid, invalid_body) = call(
        &app,
        Method::POST,
        "/api/v1/webhooks/subscriptions",
        Some(&json!({ "url": "not a url", "events": [] })),
        Some(&access),
    )
    .await;
    assert_eq!(invalid, StatusCode::BAD_REQUEST);
    assert!(invalid_body["errors"].as_array().unwrap().len() >= 2);

    let (created, body) = call(
        &app,
        Method::POST,
        "/api/v1/webhooks/subscriptions",
        Some(&json!({ "url": "https://example.com/hook", "events": ["user.created"] })),
        Some(&access),
    )
    .await;
    assert_eq!(created, StatusCode::CREATED);
    let id = body["data"]["id"].as_str().unwrap().to_owned();
    assert_eq!(body["data"]["retry_count"], 3);

    let (fetched, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/webhooks/subscriptions/{id}"),
        None,
        Some(&access),
    )
    .await;
    let (updated, update_body) = call(
        &app,
        Method::PUT,
        &format!("/api/v1/webhooks/subscriptions/{id}"),
        Some(&json!({ "isActive": false })),
        Some(&access),
    )
    .await;
    let (deliveries, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/webhooks/subscriptions/{id}/deliveries?limit=5"),
        None,
        Some(&access),
    )
    .await;
    let (retry_missing, _) = call(
        &app,
        Method::POST,
        "/api/v1/webhooks/retry",
        Some(&json!({ "deliveryId": uuid::Uuid::new_v4() })),
        Some(&access),
    )
    .await;
    let (deleted, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/webhooks/subscriptions/{id}"),
        None,
        Some(&access),
    )
    .await;
    let (gone, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/webhooks/subscriptions/{id}"),
        None,
        Some(&access),
    )
    .await;

    assert_eq!(fetched, StatusCode::OK);
    assert_eq!(updated, StatusCode::OK);
    assert_eq!(update_body["data"]["is_active"], false);
    assert_eq!(deliveries, StatusCode::OK);
    assert_eq!(retry_missing, StatusCode::NOT_FOUND);
    assert_eq!(deleted, StatusCode::OK);
    assert_eq!(gone, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn login_is_rate_limited_per_client(pool: PgPool) {
    let app = router(&pool).await;
    let attempt = json!({ "username": unique_name("Nobody Here"), "password": TEST_PASSWORD });
    let mut last = StatusCode::OK;

    for _ in 0..11 {
        last = call(
            &app,
            Method::POST,
            "/api/v1/auth/login",
            Some(&attempt),
            None,
        )
        .await
        .0;
    }

    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn logout_and_refresh_rotate_and_revoke_sessions_when_redis_is_available(pool: PgPool) {
    let redis = live_redis().await;
    if !redis.is_available() {
        return;
    }
    let app = build_application(&pool, RecordingEmailSender::new(true), redis)
        .await
        .router();
    UserFactory::new(&pool, "Session User").create().await;
    let (_, login) = call(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&json!({ "username": "Session User", "password": TEST_PASSWORD, "rememberMe": true })),
        None,
    )
    .await;
    let access = login["data"]["accessToken"].as_str().unwrap().to_owned();
    let refresh = login["data"]["refreshToken"].as_str().unwrap().to_owned();

    let (refreshed, rotated) = call(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(&json!({ "refreshToken": refresh, "rememberMe": true })),
        None,
    )
    .await;
    let (replay, _) = call(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(&json!({ "refreshToken": refresh, "rememberMe": true })),
        None,
    )
    .await;
    assert_eq!(refreshed, StatusCode::OK);
    assert_eq!(replay, StatusCode::UNAUTHORIZED);

    let new_access = rotated["data"]["accessToken"].as_str().unwrap().to_owned();
    let (logout, _) = call(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        None,
        Some(&new_access),
    )
    .await;
    let (after_old, _) = call(&app, Method::GET, "/api/v1/users", None, Some(&access)).await;
    let (after_new, _) = call(&app, Method::GET, "/api/v1/users", None, Some(&new_access)).await;

    assert_eq!(logout, StatusCode::NO_CONTENT);
    assert_eq!(after_old, StatusCode::UNAUTHORIZED);
    assert_eq!(after_new, StatusCode::UNAUTHORIZED);
}
