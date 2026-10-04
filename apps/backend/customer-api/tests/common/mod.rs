#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use customer_api::application::Application;
use customer_api::config::ServiceConfig;
use http_body_util::BodyExt;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_config::EnvReader;
use ru5ty_gate_database::sqlx::PgPool;
use ru5ty_gate_database::{seed_roles, seed_user_statuses, sqlx};
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_utilities::PasswordUtil;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

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

pub const TEST_PASSWORD: &str = "CorrectHorse1";

#[derive(Debug, Clone)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub template: String,
    pub variables: BTreeMap<String, String>,
}

pub struct RecordingEmailSender {
    pub sent: Mutex<Vec<SentEmail>>,
    pub succeed: bool,
}

impl RecordingEmailSender {
    pub fn new(succeed: bool) -> Arc<Self> {
        Arc::new(Self {
            sent: Mutex::new(Vec::new()),
            succeed,
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
        self.succeed
    }
}

pub fn test_config() -> ServiceConfig {
    let env = EnvReader::from_pairs([
        ("PORT", "4002"),
        ("APP_ENV", "development"),
        ("CORS_ORIGIN", "http://localhost:3000"),
        ("CUSTOMER_WEB_URL", "http://localhost:3000"),
        ("REDIS_URL", "redis://127.0.0.1:6379"),
        ("DATABASE_URL", "postgres://unused"),
        ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
        ("JWT_REFRESH_SECRET", "fedcba9876543210fedcba9876543210"),
    ]);
    ServiceConfig::from_env(&env).unwrap()
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
) -> Application {
    seed(pool).await;
    Application::with_parts(test_config(), pool.clone(), redis, email)
}

pub struct UserFactory<'a> {
    pool: &'a PgPool,
    username: String,
    email: String,
    role: &'static str,
    status: &'static str,
    age: i32,
    gender: Option<String>,
}

impl<'a> UserFactory<'a> {
    pub fn new(pool: &'a PgPool, username: &str) -> Self {
        Self {
            pool,
            username: username.to_owned(),
            email: format!("{}@example.com", username.to_lowercase().replace(' ', "")),
            role: "Chat User",
            status: "Verified",
            age: 30,
            gender: None,
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

    pub fn age(mut self, age: i32) -> Self {
        self.age = age;
        self
    }

    pub fn gender(mut self, gender: &str) -> Self {
        self.gender = Some(gender.to_owned());
        self
    }

    pub fn email(mut self, email: &str) -> Self {
        email.clone_into(&mut self.email);
        self
    }

    pub async fn create(self) -> Uuid {
        let hash = PasswordUtil::hash(TEST_PASSWORD).await.unwrap();
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (username, password, email, age, gender, ip_address, role_id, \
             user_status_id) VALUES ($1, $2, $3, $4, $5, '127.0.0.1', \
             (SELECT id FROM roles WHERE name = $6), \
             (SELECT id FROM user_statuses WHERE name = $7)) RETURNING id",
        )
        .bind(&self.username)
        .bind(hash)
        .bind(&self.email)
        .bind(self.age)
        .bind(self.gender.as_deref())
        .bind(self.role)
        .bind(self.status)
        .fetch_one(self.pool)
        .await
        .unwrap()
    }
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
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

pub async fn login_token(app: &Router, username: &str) -> String {
    let (status, body) = call(
        app,
        Method::POST,
        "/api/v1/auth/login",
        Some(&serde_json::json!({ "username": username, "password": TEST_PASSWORD })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"]["accessToken"].as_str().unwrap().to_owned()
}
