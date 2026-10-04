---
name: testing
description: Use when writing unit tests, integration tests, or test utilities for any Rust backend service. Covers the tests/ layout, #[sqlx::test] service tests against a real Postgres, HTTP tests through the Axum router, test factories, Redis in tests, and the lint exceptions for test code. Also use when debugging failing tests or improving test coverage.
tools: Read, Edit, Write, Grep, Glob, Bash
model: claude-haiku-4-5-20251001
---

Defaults to Haiku — writing tests against already-implemented, already-understood code following the coverage table below is largely mechanical. If the invoking session judges this instance genuinely needs deeper reasoning (a tricky concurrency/race-condition test, a novel integration scenario with no existing pattern to mirror), override back to the session's own model rather than pushing through on Haiku.

The rules in `.claude/rules/testing.md` apply to everything here.

## Test file location

```
apps/backend/[service]/
└── tests/
    ├── common/mod.rs             shared helpers, factories, recording senders
    ├── services/
    │   ├── main.rs               #[path = "../common/mod.rs"] mod common; mod user_service; ...
    │   └── user_service.rs
    ├── integration/
    │   └── main.rs               full HTTP tests through the Router
    └── unit/
        ├── main.rs
        ├── service_config.rs
        └── validation.rs
```

Each directory with a `main.rs` compiles to one test binary, so shared helpers are compiled once per binary and `#![allow(dead_code)]` sits at the top of `common/mod.rs`.

## Global test setup (`tests/common/mod.rs`)

```rust
pub fn test_config() -> ServiceConfig {
    let env = EnvReader::from_pairs([
        ("PORT", "4002"),
        ("APP_ENV", "development"),
        ("CORS_ORIGIN", "http://localhost:3000"),
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

pub async fn build_application(pool: &PgPool, email: Arc<RecordingEmailSender>, redis: RedisService) -> Application {
    seed(pool).await;
    Application::with_parts(test_config(), pool.clone(), redis, email)
}
```

`Application::with_parts` builds the full service container from a pool, a Redis handle and an `Arc<dyn EmailSender>` — the three things tests control.

## Test factories

Builders insert real rows and return the new id:

```rust
pub struct UserFactory<'a> { pool: &'a PgPool, username: String, role: &'static str, status: &'static str, age: i32 }

impl<'a> UserFactory<'a> {
    pub fn new(pool: &'a PgPool, username: &str) -> Self { ... }
    pub fn role(mut self, role: &'static str) -> Self { self.role = role; self }
    pub async fn create(self) -> Uuid {
        let hash = PasswordUtil::hash(TEST_PASSWORD).await.unwrap();
        sqlx::query_scalar::<_, Uuid>("INSERT INTO users (...) VALUES (...) RETURNING id")
            .bind(&self.username)
            .bind(hash)
            .fetch_one(self.pool)
            .await
            .unwrap()
    }
}
```

Use `unique_name("Prefix")` for usernames in tests that share Redis state, so lockout counters never collide.

## Service tests

```rust
#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn filters_by_gender_age_and_pages_results(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.user;
    UserFactory::new(&pool, "Young Female").age(20).gender("female").create().await;

    let females = service
        .get_users(&UserFilters { gender: Some("female".to_owned()), ..UserFilters::default() }, None)
        .await
        .unwrap();

    assert_eq!(females.len(), 1);
}
```

`#[sqlx::test]` creates a fresh database per test and drops it afterwards — no cleanup code, no shared state.

## Integration tests — routes

```rust
pub async fn call(app: &Router, method: Method, uri: &str, body: Option<&Value>, token: Option<&str>) -> (StatusCode, Value)
```

`call` builds the request, runs it with `app.clone().oneshot(request)`, collects the body and parses JSON. `login_token(&app, "Some User")` logs in through `/api/v1/auth/login` and returns the access token.

```rust
#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn users_endpoint_requires_a_token(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let router = app.router();

    let (status, body) = call(&router, Method::GET, "/api/v1/users", None, None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["isSuccessful"], false);
}
```

## Replacing external services

- Email: `RecordingEmailSender` implements `EmailSender`, records `SentEmail { to, subject, template, variables }`, and returns the configured success flag
- Webhook subscribers: a local Axum server on `127.0.0.1:0` that records headers and bodies (`start_receiver` in `schedule-api/tests/common`)
- Time: pass explicit timestamps into SQL (`next_retry_at <= NOW()`) or set rows to past values instead of sleeping

## What to test per layer

| Layer | Must cover |
|---|---|
| Service | each public method: success, not found, conflict/duplicate, business-rule failure, soft delete, audit columns |
| Route | 401 without token, 403 without permission, 400 on invalid and unknown fields, 404, happy path with the envelope shape |
| Config | complete environment loads, each required value missing fails, bad numbers fail |
| Validation | each `#[validate]` rule accepts and rejects the boundary values, `deny_unknown_fields` rejects extras |
| Auth | login, lockout, refresh rotation and replay, logout revoking other live tokens, MFA two-step |

## Rules

Never use `sleep` to wait for async work — poll with a bounded loop or inject the dependency. Never share rows between tests. Never hit external networks. Test files start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` and production code never uses that attribute. Test names are plain-English `snake_case` sentences. Run `cargo test -p ru5ty-gate-<service>` with `DATABASE_URL` and `TEST_REDIS_URL` set, then `cargo clippy --workspace --all-targets -- -D warnings` — clippy checks test code too.
