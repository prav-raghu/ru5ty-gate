#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use admin_api::application::Application;
use admin_api::config::ServiceConfig;
use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_config::EnvReader;
use ru5ty_gate_database::sqlx::PgPool;
use ru5ty_gate_database::{seed_roles, seed_user_statuses, sqlx};
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_utilities::PasswordUtil;
use serde_json::Value;
use totp_rs::{Builder, Secret};
use tower::ServiceExt;
use uuid::Uuid;

pub const TEST_PASSWORD: &str = "CorrectHorse1";
pub const TWO_FACTOR_KEY: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

#[derive(Debug, Clone)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub template: String,
    pub variables: BTreeMap<String, String>,
}

pub struct RecordingEmailSender {
    pub sent: Mutex<Vec<SentEmail>>,
}

impl RecordingEmailSender {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sent: Mutex::new(Vec::new()),
        })
    }

    pub fn last(&self) -> Option<SentEmail> {
        self.sent.lock().unwrap().last().cloned()
    }

    pub fn count(&self) -> usize {
        self.sent.lock().unwrap().len()
    }
}

#[async_trait]
impl EmailSender for RecordingEmailSender {
    async fn send_mail(
        &self,
        to: &str,
        subject: &str,
        template: &str,
        variables: &BTreeMap<String, String>,
    ) -> bool {
        self.sent.lock().unwrap().push(SentEmail {
            to: to.to_owned(),
            subject: subject.to_owned(),
            template: template.to_owned(),
            variables: variables.clone(),
        });
        true
    }
}

pub fn config_with(extra: &[(&str, &str)]) -> ServiceConfig {
    let mut pairs = vec![
        ("PORT", "4001"),
        ("APP_ENV", "development"),
        ("CORS_ORIGIN", "http://localhost:4004"),
        ("ADMIN_WEB_URL", "http://localhost:4004"),
        ("REDIS_URL", "redis://127.0.0.1:6379"),
        ("DATABASE_URL", "postgres://unused"),
        ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
        ("JWT_REFRESH_SECRET", "fedcba9876543210fedcba9876543210"),
        ("TWO_FACTOR_ENCRYPTION_KEY", TWO_FACTOR_KEY),
    ];
    pairs.extend_from_slice(extra);
    ServiceConfig::from_env(&EnvReader::from_pairs(pairs)).unwrap()
}

pub async fn live_redis() -> RedisService {
    match std::env::var("TEST_REDIS_URL") {
        Ok(url) => RedisService::connect(&url, true).await,
        Err(_) => RedisService::disabled(),
    }
}

pub async fn seed(pool: &PgPool) {
    seed_roles(pool).await.unwrap();
    seed_user_statuses(pool).await.unwrap();
}

pub async fn build_application(
    pool: &PgPool,
    email: Arc<RecordingEmailSender>,
    redis: RedisService,
    config: ServiceConfig,
) -> Application {
    seed(pool).await;
    Application::with_parts(config, pool.clone(), redis, email).unwrap()
}

pub fn unique_name(prefix: &str) -> String {
    let suffix: String = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .map(|digit| match digit.to_digit(16) {
            Some(value) => char::from(b'a' + u8::try_from(value).unwrap_or(0)),
            None => 'z',
        })
        .collect();
    format!("{prefix} {suffix}")
}

pub struct UserFactory<'a> {
    pool: &'a PgPool,
    username: String,
    email: String,
    role: &'static str,
    status: &'static str,
    allow_email: bool,
}

impl<'a> UserFactory<'a> {
    pub fn new(pool: &'a PgPool, username: &str) -> Self {
        Self {
            pool,
            username: username.to_owned(),
            email: format!("{}@example.com", username.to_lowercase().replace(' ', "")),
            role: "Super Admin",
            status: "Online",
            allow_email: false,
        }
    }

    pub fn role(mut self, role: &'static str) -> Self {
        self.role = role;
        self
    }

    pub fn status(mut self, status: &'static str) -> Self {
        self.status = status;
        self
    }

    pub fn allow_email(mut self) -> Self {
        self.allow_email = true;
        self
    }

    pub async fn create(self) -> (Uuid, String) {
        let hash = PasswordUtil::hash(TEST_PASSWORD).await.unwrap();
        let id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (username, password, email, ip_address, role_id, user_status_id, \
             allow_email_communications) VALUES ($1, $2, $3, '127.0.0.1', \
             (SELECT id FROM roles WHERE name = $4), \
             (SELECT id FROM user_statuses WHERE name = $5), $6) RETURNING id",
        )
        .bind(&self.username)
        .bind(hash)
        .bind(&self.email)
        .bind(self.role)
        .bind(self.status)
        .bind(self.allow_email)
        .fetch_one(self.pool)
        .await
        .unwrap();
        (id, self.email)
    }
}

pub fn current_code(secret_base32: &str) -> String {
    let secret = Secret::try_from_base32(secret_base32).unwrap();
    Builder::new()
        .with_secret(secret)
        .build()
        .unwrap()
        .generate_current()
        .to_string()
}

pub async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<&Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let request = builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

pub async fn login_token(app: &Router, email: &str) -> String {
    let (status, body) = call(
        app,
        Method::POST,
        "/api/v1/auth/login",
        Some(
            &serde_json::json!({ "email": email, "password": TEST_PASSWORD, "rememberMe": false }),
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"]["authToken"].as_str().unwrap().to_owned()
}
