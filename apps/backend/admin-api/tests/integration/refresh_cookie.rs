use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use ru5ty_gate_database::sqlx::{self, PgPool};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, config_with, live_redis,
    unique_name,
};

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

impl Reply {
    fn set_cookie(&self) -> String {
        self.headers
            .get(header::SET_COOKIE)
            .map(|value| value.to_str().unwrap().to_owned())
            .unwrap_or_default()
    }

    fn cookie_value(&self) -> String {
        self.set_cookie()
            .split(';')
            .next()
            .unwrap()
            .trim_start_matches("rg_refresh=")
            .to_owned()
    }
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
    token: Option<&str>,
) -> Reply {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("rg_refresh={cookie}"));
    }
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        headers,
        body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    }
}

async fn app_with(pool: &PgPool, extra: &[(&str, &str)]) -> Router {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(extra),
    )
    .await
    .router()
}

async fn login(app: &Router, pool: &PgPool, remember_me: bool) -> Reply {
    let (_, email) = UserFactory::new(pool, &unique_name("Cookie Admin"))
        .create()
        .await;
    send(
        app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({ "email": email, "password": TEST_PASSWORD, "rememberMe": remember_me })),
        None,
        None,
    )
    .await
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn login_sets_an_http_only_strict_refresh_cookie(pool: PgPool) {
    let app = app_with(&pool, &[]).await;

    let short = login(&app, &pool, false).await;
    let long = login(&app, &pool, true).await;

    assert_eq!(short.status, StatusCode::OK);
    let cookie = short.set_cookie();
    assert!(cookie.starts_with("rg_refresh="), "{cookie}");
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(cookie.contains("Path=/api/v1/auth"));
    assert!(cookie.contains("Max-Age=86400"));
    assert!(!cookie.contains("Secure"));
    assert!(long.set_cookie().contains("Max-Age=2592000"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn production_cookies_are_secure(pool: PgPool) {
    let app = app_with(
        &pool,
        &[
            ("APP_ENV", "production"),
            ("TRUSTED_PROXY_HOPS", "0"),
            ("ADMIN_BOOTSTRAP_ENABLED", "false"),
        ],
    )
    .await;

    let reply = login(&app, &pool, false).await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.set_cookie().contains("; Secure"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn a_failed_login_sets_no_cookie(pool: PgPool) {
    let app = app_with(&pool, &[]).await;

    let reply = send(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(
            json!({ "email": "ghost@example.com", "password": TEST_PASSWORD, "rememberMe": false }),
        ),
        None,
        None,
    )
    .await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.headers.get(header::SET_COOKIE).is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn refresh_without_any_token_is_rejected_and_clears_the_cookie(pool: PgPool) {
    let app = app_with(&pool, &[]).await;

    let reply = send(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({})),
        None,
        None,
    )
    .await;

    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert!(reply.set_cookie().contains("Max-Age=0"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn refresh_reads_the_cookie_rotates_it_and_rejects_reuse(pool: PgPool) {
    if !live_redis().await.is_available() {
        return;
    }
    let app = app_with(&pool, &[]).await;
    let first = login(&app, &pool, false).await;
    let original = first.cookie_value();

    let refreshed = send(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({})),
        Some(&original),
        None,
    )
    .await;
    let access = refreshed.body["data"]["accessToken"].as_str().unwrap();
    let me = send(
        &app,
        Method::GET,
        "/api/v1/auth/me",
        None,
        None,
        Some(access),
    )
    .await;
    let replay = send(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({})),
        Some(&original),
        None,
    )
    .await;

    assert_eq!(refreshed.status, StatusCode::OK, "{}", refreshed.body);
    assert_ne!(refreshed.cookie_value(), original);
    assert!(refreshed.set_cookie().contains("HttpOnly"));
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(replay.status, StatusCode::UNAUTHORIZED);
    assert!(replay.set_cookie().contains("Max-Age=0"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn refresh_still_accepts_a_token_in_the_body(pool: PgPool) {
    if !live_redis().await.is_available() {
        return;
    }
    let app = app_with(&pool, &[]).await;
    let first = login(&app, &pool, false).await;
    let token = first.body["data"]["refreshToken"].as_str().unwrap();

    let reply = send(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({ "refreshToken": token, "rememberMe": false })),
        None,
        None,
    )
    .await;

    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn logout_clears_the_cookie(pool: PgPool) {
    let app = app_with(&pool, &[]).await;
    let first = login(&app, &pool, false).await;
    let token = first.body["data"]["authToken"].as_str().unwrap();

    let reply = send(
        &app,
        Method::GET,
        "/api/v1/auth/logout",
        None,
        None,
        Some(token),
    )
    .await;

    assert_eq!(reply.status, StatusCode::OK);
    let cookie = reply.set_cookie();
    assert!(cookie.starts_with("rg_refresh=;"), "{cookie}");
    assert!(cookie.contains("Max-Age=0"));
}
