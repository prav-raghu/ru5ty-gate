#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../common/mod.rs"]
mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ru5ty_gate_database::sqlx::{self, PgPool};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, call, config_with,
    live_redis, login_token, unique_name,
};

async fn router(pool: &PgPool) -> axum::Router {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await
    .router()
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn public_endpoints_and_cross_cutting_behaviour(pool: PgPool) {
    let app = router(&pool).await;

    let (ping, ping_body) = call(&app, Method::GET, "/api/v1/ping", None, None).await;
    let (ready, _) = call(&app, Method::GET, "/api/v1/ready", None, None).await;
    let (missing, missing_body) = call(&app, Method::GET, "/api/v1/nope", None, None).await;
    let response = app
        .clone()
        .oneshot(Request::get("/api/v2/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let v2: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(ping, StatusCode::OK);
    assert_eq!(ping_body["status"], "pong");
    assert!(ping_body["responseDateTime"].is_string());
    assert_eq!(ready, StatusCode::OK);
    assert_eq!(missing, StatusCode::NOT_FOUND);
    assert_eq!(missing_body["isSuccessful"], false);
    assert_eq!(v2["version"], "v2");
    assert_eq!(headers["x-api-version"], "v2");
    assert_eq!(headers["x-frame-options"], "SAMEORIGIN");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn login_me_and_protected_route_flow(pool: PgPool) {
    let app = router(&pool).await;
    let name = unique_name("Flow Admin");
    let (_, address) = UserFactory::new(&pool, &name).create().await;

    let (anonymous, _) = call(&app, Method::GET, "/api/v1/auth/me", None, None).await;
    let (invalid, validation) = call(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&json!({ "email": "nope", "password": "x", "rememberMe": false })),
        None,
    )
    .await;
    let token = login_token(&app, &address).await;
    let (me_status, me) = call(&app, Method::GET, "/api/v1/auth/me", None, Some(&token)).await;
    let (roles_status, roles) =
        call(&app, Method::GET, "/api/v1/users/roles", None, Some(&token)).await;

    assert_eq!(anonymous, StatusCode::UNAUTHORIZED);
    assert_eq!(invalid, StatusCode::BAD_REQUEST);
    let fields: Vec<_> = validation["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|error| error["field"].as_str().unwrap())
        .collect();
    assert!(fields.contains(&"email") && fields.contains(&"password"));
    assert_eq!(me_status, StatusCode::OK);
    assert_eq!(me["username"], name.as_str());
    assert_eq!(me["roles"]["name"], "Super Admin");
    assert_eq!(roles_status, StatusCode::OK);
    assert_eq!(roles["data"].as_array().unwrap().len(), 4);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn customer_tokens_and_tokens_for_offline_users_are_rejected(pool: PgPool) {
    let app = router(&pool).await;
    let (_, address) = UserFactory::new(&pool, &unique_name("Soon Offline"))
        .create()
        .await;
    let token = login_token(&app, &address).await;
    assert_eq!(
        call(&app, Method::GET, "/api/v1/auth/me", None, Some(&token))
            .await
            .0,
        StatusCode::OK
    );

    sqlx::query(
        "UPDATE users SET user_status_id = (SELECT id FROM user_statuses WHERE name = 'Offline')",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        call(&app, Method::GET, "/api/v1/auth/me", None, Some(&token))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn permissions_gate_each_route_group(pool: PgPool) {
    let app = router(&pool).await;
    let (_, support_email) = UserFactory::new(&pool, &unique_name("Support Agent"))
        .role("Support")
        .create()
        .await;
    let (_, moderator_email) = UserFactory::new(&pool, &unique_name("Moderator Agent"))
        .role("Moderator")
        .create()
        .await;
    let support = login_token(&app, &support_email).await;
    let moderator = login_token(&app, &moderator_email).await;

    let (support_reports, _) = call(
        &app,
        Method::GET,
        "/api/v1/reports/system-metrics",
        None,
        Some(&support),
    )
    .await;
    let (support_export, _) = call(
        &app,
        Method::POST,
        "/api/v1/reports/generate",
        Some(&json!({ "type": "USER_ACTIVITY", "format": "CSV" })),
        Some(&support),
    )
    .await;
    let (support_batch, _) = call(
        &app,
        Method::POST,
        "/api/v1/batch/custom",
        Some(&json!({ "operation": "CREATE", "items": [{ "a": 1 }] })),
        Some(&support),
    )
    .await;
    let (support_onboard, _) = call(
        &app,
        Method::POST,
        "/api/v1/users/onboarding",
        Some(&json!({})),
        Some(&support),
    )
    .await;
    let (moderator_batch, batch) = call(
        &app,
        Method::POST,
        "/api/v1/batch/custom",
        Some(&json!({ "operation": "CREATE", "items": [{ "a": 1 }] })),
        Some(&moderator),
    )
    .await;

    assert_eq!(support_reports, StatusCode::OK);
    assert_eq!(support_export, StatusCode::FORBIDDEN);
    assert_eq!(support_batch, StatusCode::FORBIDDEN);
    assert_eq!(support_onboard, StatusCode::FORBIDDEN);
    assert_eq!(moderator_batch, StatusCode::OK);
    assert_eq!(batch["data"]["successful"], 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn bootstrap_route_creates_one_admin_then_returns_403(pool: PgPool) {
    let app = router(&pool).await;
    let payload =
        json!({ "username": "root admin", "email": "root@example.com", "password": TEST_PASSWORD });

    let (first, _) = call(
        &app,
        Method::POST,
        "/api/v1/auth/bootstrap-admin",
        Some(&payload),
        None,
    )
    .await;
    let (second, second_body) = call(
        &app,
        Method::POST,
        "/api/v1/auth/bootstrap-admin",
        Some(&json!({ "username": "other admin", "email": "other@example.com", "password": TEST_PASSWORD })),
        None,
    )
    .await;
    let (weak, _) = call(
        &app,
        Method::POST,
        "/api/v1/auth/bootstrap-admin",
        Some(&json!({ "username": "x", "email": "bad", "password": "short" })),
        None,
    )
    .await;

    assert_eq!(first, StatusCode::CREATED);
    assert_eq!(second, StatusCode::FORBIDDEN);
    assert_eq!(
        second_body["message"],
        "An administrator account already exists"
    );
    assert_eq!(weak, StatusCode::BAD_REQUEST);
    let token = login_token(&app, "root@example.com").await;
    assert!(!token.is_empty());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn batch_endpoints_return_207_for_partial_failures(pool: PgPool) {
    let app = router(&pool).await;
    let (_, address) = UserFactory::new(&pool, &unique_name("Batch Admin"))
        .create()
        .await;
    let token = login_token(&app, &address).await;

    let (created, created_body) = call(
        &app,
        Method::POST,
        "/api/v1/batch/users/create",
        Some(&json!({ "users": [
            { "email": "a1@example.com", "name": "Batch Alpha", "password": TEST_PASSWORD },
            { "email": "a2@example.com", "name": "Batch Alpha", "password": TEST_PASSWORD }
        ]})),
        Some(&token),
    )
    .await;
    let (empty, _) = call(
        &app,
        Method::POST,
        "/api/v1/batch/users/create",
        Some(&json!({ "users": [] })),
        Some(&token),
    )
    .await;
    let (invalid_item, invalid_body) = call(
        &app,
        Method::POST,
        "/api/v1/batch/users/create",
        Some(&json!({ "users": [{ "email": "bad", "name": "x", "password": "short" }] })),
        Some(&token),
    )
    .await;

    assert_eq!(created, StatusCode::MULTI_STATUS);
    assert_eq!(created_body["isSuccessful"], false);
    assert_eq!(created_body["data"]["failed"], 1);
    assert_eq!(empty, StatusCode::BAD_REQUEST);
    assert_eq!(invalid_item, StatusCode::BAD_REQUEST);
    assert!(
        invalid_body["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["field"].as_str().unwrap().starts_with("users[0]"))
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn report_endpoints_serve_json_and_downloadable_streams(pool: PgPool) {
    let app = router(&pool).await;
    let (_, address) = UserFactory::new(&pool, &unique_name("Report Admin"))
        .create()
        .await;
    let token = login_token(&app, &address).await;

    let (status, activity) = call(
        &app,
        Method::GET,
        "/api/v1/reports/user-activity",
        None,
        Some(&token),
    )
    .await;
    let (generated_status, generated) = call(
        &app,
        Method::POST,
        "/api/v1/reports/generate",
        Some(&json!({ "type": "USER_ACTIVITY", "format": "CSV" })),
        Some(&token),
    )
    .await;
    let (bad, _) = call(
        &app,
        Method::GET,
        "/api/v1/reports/user-activity?bogus=1",
        None,
        Some(&token),
    )
    .await;
    let streamed = app
        .clone()
        .oneshot(
            Request::get("/api/v1/reports/stream?type=USER_ACTIVITY&format=csv")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let disposition = streamed.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .to_owned();
    let content_type = streamed.headers()["content-type"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = streamed.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(activity["data"]["recordCount"], 1);
    assert_eq!(generated_status, StatusCode::OK);
    assert_eq!(generated["isSuccessful"], true);
    assert_eq!(bad, StatusCode::BAD_REQUEST);
    assert!(disposition.contains("report-USER_ACTIVITY-"));
    assert_eq!(content_type, "text/csv");
    assert!(String::from_utf8_lossy(&bytes).contains("userId"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn logins_are_rate_limited_and_logout_revokes_the_session_when_redis_is_available(
    pool: PgPool,
) {
    let app = router(&pool).await;
    let attempt =
        json!({ "email": "ghost@example.com", "password": TEST_PASSWORD, "rememberMe": false });
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

    let redis = live_redis().await;
    if !redis.is_available() {
        return;
    }
    let session_app =
        build_application(&pool, RecordingEmailSender::new(), redis, config_with(&[]))
            .await
            .router();
    let (_, address) = UserFactory::new(&pool, &unique_name("Session Admin"))
        .create()
        .await;
    let token = login_token(&session_app, &address).await;

    let (logout, body) = call(
        &session_app,
        Method::GET,
        "/api/v1/auth/logout",
        None,
        Some(&token),
    )
    .await;
    let (after, _) = call(
        &session_app,
        Method::GET,
        "/api/v1/auth/me",
        None,
        Some(&token),
    )
    .await;

    assert_eq!(logout, StatusCode::OK);
    assert_eq!(body["message"], "Successfully Logged Out");
    assert_eq!(after, StatusCode::UNAUTHORIZED);
}
