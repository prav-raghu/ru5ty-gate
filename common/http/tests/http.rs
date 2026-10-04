#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, post};
use http_body_util::BodyExt;
use ru5ty_gate_http::{
    ApiPath, AppError, AuthUser, Authenticator, RateLimitPolicy, RateLimiter, ValidatedJson,
    api_version, authenticate, not_found, rate_limit, require_permission, response_timestamp,
    security_headers,
};
use ru5ty_gate_types::{Permission, TokenScope};
use serde::Deserialize;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Payload {
    #[validate(email(message = "Must be a valid email address"))]
    email: String,
    #[validate(length(min = 8, max = 20))]
    password: String,
    #[validate(range(min = 13, max = 120))]
    age: Option<u8>,
    #[validate(length(min = 2))]
    first_name: Option<String>,
}

async fn create(ValidatedJson(payload): ValidatedJson<Payload>) -> Result<String, AppError> {
    Ok(payload.email)
}

async fn by_id(ApiPath(id): ApiPath<Uuid>) -> String {
    id.to_string()
}

async fn send(app: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn json_post(uri: &str, body: &Value) -> Request<Body> {
    Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn app() -> Router {
    Router::new()
        .route("/create", post(create))
        .route("/items/{id}", get(by_id))
        .fallback(not_found)
}

#[tokio::test]
async fn valid_payload_passes() {
    let (status, _) = send(
        app(),
        json_post(
            "/create",
            &json!({"email": "a@b.com", "password": "longenough"}),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn validation_failures_use_the_field_error_envelope() {
    let (status, body) = send(
        app(),
        json_post("/create", &json!({"email": "nope", "password": "short"})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["isSuccessful"], false);
    assert_eq!(body["message"], "Validation failed");
    let errors = body["errors"].as_array().unwrap();
    assert!(
        errors.iter().any(|error| error["field"] == "email"
            && error["message"] == "Must be a valid email address")
    );
    assert!(
        errors.iter().any(|error| error["field"] == "password"
            && error["message"] == "Must be at least 8 characters")
    );
}

#[tokio::test]
async fn missing_and_unknown_fields_are_reported_per_field() {
    let (_, missing) = send(app(), json_post("/create", &json!({"email": "a@b.com"}))).await;
    let (_, unknown) = send(
        app(),
        json_post(
            "/create",
            &json!({"email": "a@b.com", "password": "longenough", "extra": 1}),
        ),
    )
    .await;

    assert_eq!(missing["errors"][0]["field"], "password");
    assert_eq!(missing["errors"][0]["message"], "Is required");
    assert_eq!(unknown["errors"][0]["field"], "extra");
    assert_eq!(unknown["errors"][0]["message"], "Unexpected property");
}

#[tokio::test]
async fn multi_word_fields_are_reported_in_camel_case() {
    let (_, body) = send(
        app(),
        json_post(
            "/create",
            &json!({"email": "a@b.com", "password": "longenough", "firstName": "x"}),
        ),
    )
    .await;

    assert_eq!(body["errors"][0]["field"], "firstName");
}

#[tokio::test]
async fn range_errors_are_reported() {
    let (_, body) = send(
        app(),
        json_post(
            "/create",
            &json!({"email": "a@b.com", "password": "longenough", "age": 5}),
        ),
    )
    .await;

    assert_eq!(body["errors"][0]["field"], "age");
    assert_eq!(body["errors"][0]["message"], "Must be between 13 and 120");
}

#[tokio::test]
async fn malformed_json_is_a_bad_request() {
    let request = Request::post("/create")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();

    let (status, body) = send(app(), request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["isSuccessful"], false);
}

#[tokio::test]
async fn invalid_path_params_and_unknown_routes_use_the_envelope() {
    let (bad_status, bad_body) = send(
        app(),
        Request::get("/items/not-a-uuid")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let (missing_status, missing_body) =
        send(app(), Request::get("/nope").body(Body::empty()).unwrap()).await;

    assert_eq!(bad_status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_body["errors"][0]["field"], "params");
    assert_eq!(missing_status, StatusCode::NOT_FOUND);
    assert_eq!(missing_body["message"], "Not found");
}

struct StaticAuthenticator;

#[async_trait]
impl Authenticator for StaticAuthenticator {
    async fn authenticate(&self, bearer_token: &str) -> Option<AuthUser> {
        let permissions = match bearer_token {
            "admin" => vec![Permission::BatchWrite],
            "reader" => vec![Permission::UserRead],
            _ => return None,
        };
        Some(AuthUser {
            id: Uuid::nil(),
            username: "tester".to_owned(),
            email: None,
            role: "Moderator".to_owned(),
            permissions,
            scope: TokenScope::Admin,
        })
    }
}

fn protected_app() -> Router {
    let authenticator: Arc<dyn Authenticator> = Arc::new(StaticAuthenticator);
    let guarded = Router::new()
        .route("/batch", get(|| async { "ok" }))
        .route_layer(from_fn_with_state(
            Permission::BatchWrite,
            require_permission,
        ))
        .route_layer(from_fn_with_state(authenticator, authenticate));
    Router::new()
        .route("/public", get(|| async { "open" }))
        .merge(guarded)
}

fn get_with_token(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::get(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn auth_middleware_distinguishes_401_and_403() {
    let (anonymous, anonymous_body) = send(protected_app(), get_with_token("/batch", None)).await;
    let (bad_token, _) = send(protected_app(), get_with_token("/batch", Some("wrong"))).await;
    let (forbidden, forbidden_body) =
        send(protected_app(), get_with_token("/batch", Some("reader"))).await;
    let (allowed, _) = send(protected_app(), get_with_token("/batch", Some("admin"))).await;
    let (public, _) = send(protected_app(), get_with_token("/public", None)).await;

    assert_eq!(anonymous, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous_body["message"], "Unauthorized");
    assert_eq!(bad_token, StatusCode::UNAUTHORIZED);
    assert_eq!(forbidden, StatusCode::FORBIDDEN);
    assert_eq!(
        forbidden_body["message"],
        "Forbidden: insufficient permissions"
    );
    assert_eq!(allowed, StatusCode::OK);
    assert_eq!(public, StatusCode::OK);
}

#[test]
fn rate_limiter_blocks_after_max_and_recovers_after_the_window() {
    let limiter = RateLimiter::new(RateLimitPolicy {
        max: 2,
        window: Duration::from_secs(60),
        message: "slow down".to_owned(),
    });
    let start = Instant::now();

    assert!(limiter.check_at("ip:1", start).is_ok());
    assert!(limiter.check_at("ip:1", start).is_ok());
    assert!(limiter.check_at("ip:1", start).is_err());
    assert!(limiter.check_at("ip:2", start).is_ok());
    assert!(
        limiter
            .check_at("ip:1", start + Duration::from_secs(61))
            .is_ok()
    );
}

#[tokio::test]
async fn rate_limit_middleware_returns_429_with_retry_after() {
    let limiter = RateLimiter::new(RateLimitPolicy::per_minute(
        1,
        "Too many authentication attempts.",
    ));
    let app = Router::new()
        .route("/auth", get(|| async { "ok" }))
        .route_layer(from_fn_with_state(limiter, rate_limit));

    let (first, _) = send(app.clone(), get_with_token("/auth", None)).await;
    let response = app.oneshot(get_with_token("/auth", None)).await.unwrap();

    assert_eq!(first, StatusCode::OK);
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key("retry-after"));
}

#[tokio::test]
async fn api_version_adds_headers_and_rejects_unsupported_versions() {
    let app = Router::new()
        .route("/api/v1/ping", get(|| async { "pong" }))
        .layer(from_fn(api_version));

    let ok = app
        .clone()
        .oneshot(get_with_token("/api/v1/ping", None))
        .await
        .unwrap();
    let rejected = app
        .oneshot(
            Request::get("/api/v1/ping")
                .header("api-version", "v9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(ok.headers()["x-api-version"], "v1");
    assert_eq!(ok.headers()["deprecation"], "true");
    assert_eq!(ok.headers()["sunset"], "2026-12-31");
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn security_headers_are_applied() {
    let app = Router::new()
        .route("/", get(|| async { "x" }))
        .layer(from_fn(security_headers));

    let response = app.oneshot(get_with_token("/", None)).await.unwrap();

    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["x-frame-options"], "SAMEORIGIN");
}

#[tokio::test]
async fn response_timestamp_is_added_to_json_objects() {
    let app = Router::new()
        .route(
            "/",
            get(|| async { axum::Json(json!({"isSuccessful": true})) }),
        )
        .layer(from_fn(response_timestamp));

    let (status, body) = send(app, get_with_token("/", None)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["isSuccessful"], true);
    assert!(body["responseDateTime"].as_str().unwrap().ends_with('Z'));
}
