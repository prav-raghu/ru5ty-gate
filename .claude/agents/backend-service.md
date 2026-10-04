---
name: backend-service
description: Use when working on any Rust backend service including api-gateway, customer-api, admin-api, or schedule-api. Covers controllers, services, routes, validator request schemas, DTOs, middleware, guards, authentication, rate limiting, error handling, and general backend business logic. Also activates for refactoring existing services or debugging backend issues. For generating full CRUD endpoints from a database table, use api-builder instead.
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

## Service registry

| Service | Port | Responsibility |
|---|---|---|
| `api-gateway` | 4000 | Public entry point, reverse proxy to services, health aggregation, optional GraphQL |
| `customer-api` | 4002 | Customer-facing business logic |
| `admin-api` | 4001 | Administrative operations, owns database migrations |
| `schedule-api` | 4003 | Scheduling, cron jobs, webhook delivery processing |

## See also (deep-dive reference docs, read by cross-reference)

- `.claude/instructions/api-versioning.instruction.md` — the `/api/v1/` versioning strategy used across `customer-api`, `admin-api`, `schedule-api`
- `.claude/instructions/graphql.instruction.md` — `api-gateway`'s optional async-graphql layer over the REST services
- `.claude/instructions/batch-reporting.instruction.md` — `admin-api`'s batch operations and reporting endpoints
- `.claude/instructions/export.instruction.md` — CSV/Excel export via `ru5ty-gate-export`
- `.claude/instructions/env-config.instructions.md` — typed `ServiceConfig` through `EnvReader`
- `.claude/rules/backend.md` — the auto-loaded rules for `apps/backend/**`

## Directory structure

Every backend service follows this exact structure (see `.claude/rules/backend.md` for the annotated tree):

```
apps/backend/[service-name]/
├── Cargo.toml
├── Dockerfile
├── src/
│   ├── main.rs  lib.rs  application.rs
│   ├── config/  controllers/  services/  routes/{v1,v2}/  schemas/  dtos/
│   ├── plugins/  guards/  types/
└── tests/{common,services,integration,unit}/
```

Do not alter this structure or naming conventions unless explicitly instructed. Each folder has a `mod.rs` that declares and re-exports its files; each struct, enum, trait and constant lives in its own file.

## Architecture rules

No DI framework — `plugins/services.rs::build_services` instantiates every service once and stores them in `Services`, which lives in `Arc` inside `AppState`. Controllers are unit structs with async associated functions. Services own their dependencies (`PgPool`, `RedisService`, `Arc<dyn EmailSender>`) and are `Clone` where handlers need them. DTOs and schemas are plain structs, never `serde_json::Value` passed around untyped.

## Sentry / observability startup checklist

`ru5ty-gate-observability` exports `init_sentry(&EnvReader)` and `capture_error`. It returns `None` (a no-op) when `SENTRY_DSN` is unset, so local dev is unaffected.

- `init_sentry` is called first in `main.rs`, and the returned guard is held for the lifetime of the process (`let _sentry = ...`).
- 5xx errors reach Sentry through `AppError::Internal`'s `IntoResponse`, which logs once and calls `capture_error`. Do not capture in controllers or services; return `AppError::internal(error)` and let the response layer report it.
- Panics are captured by Sentry's panic integration and turned into 500 responses by `catch_panic_layer`.

## Controller pattern

```rust
pub struct UsersController;

impl UsersController {
    pub async fn get_user(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<UserPath>,
    ) -> Result<Response, AppError> {
        match state.services.user.get_user_by_id(path.user_id).await? {
            Some(user) => Ok((StatusCode::OK, Json(ApiResponse::success(user))).into_response()),
            None => Err(AppError::NotFound("User not found".to_owned())),
        }
    }
}
```

Controllers extract, call exactly one service method, and map the result to the envelope. They never touch `PgPool` and never contain business rules. Errors are values — no `try`/`catch`; `?` propagates `AppError`.

## Service pattern

```rust
#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<UserSummary>, AppError> {
        let record = UserRepository::find_active_summary(&self.pool, user_id)
            .await
            .map_err(AppError::internal)?;
        Ok(record.map(UserSummary::from))
    }
}
```

Services hold no SQL. Queries live in `common/database/src/repositories/` (generic `Repository::<Model>` for plain CRUD, `{Entity}Repository` for table-specific queries), return `*Record` structs, and the service maps them to DTOs with `From` (`impl From<UserSummaryRecord> for UserSummary` in `dtos/user_dto.rs`). See `rules/backend.md` (SQL access) and `documentation/repository-layer.md`.

Cache-aside on read-heavy methods uses `RedisService::get_json`/`set_json` with a TTL and deletes the key on writes. A `RedisService::disabled()` instance makes every cache call a no-op, so services never branch on whether Redis is configured.

## Request validation — `validator`, never hand-rolled

Define in `schemas/`:

```rust
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[validate(length(min = 2, max = 100))]
    pub name: String,
}
```

Handlers take `ValidatedJson<CreateUserRequest>`; it runs `serde_path_to_error` and `validate()` and returns a field-level 400 with the shared envelope. Responses are separate structs in `dtos/` with `#[serde(rename_all = "camelCase")]`.

## Route registration

```rust
pub struct UsersRoutes;

impl UsersRoutes {
    pub fn protected() -> Router<AppState> {
        Router::new()
            .route("/users", get(UsersController::get_users))
            .route("/users/{userId}", get(UsersController::get_user))
    }
}
```

Path parameters use the Axum 0.8 `{name}` syntax. `V1Routes::register` merges every entity's `public()` router and its `protected()` routers, then applies `authenticate` to the protected group with `route_layer`. Permission checks use `require_permission(Permission::...)` on the specific route.

## Security rules

JWT authentication through `TokenService` and the `authenticate` layer. Rate limiting through `RateLimiter` with per-route `RateLimitPolicy`. All inputs validated by `validator`. CORS limited to `CORS_ORIGIN` (one origin or a comma-separated list). Security headers applied by `security_headers`. Never allow auth to be bypassed via query params, headers, or flags. All secrets come from environment variables.

## Enterprise scale (1M+ concurrent users)

Stateless services — no in-memory sessions or local state beyond caches that are safe to lose. Horizontally scalable — every service works with N replicas behind a load balancer. Cache-first reads. Queue-backed writes — heavy I/O goes through `ru5ty-gate-queue` or the webhook delivery tables, never inline in handlers. Pool size via `DATABASE_CONNECTION_LIMIT` (default 10 dev, 50–100 prod).

Required endpoints on every service: public `/health` (liveness) and `/ready` (database and Redis). The `<binary> healthcheck` subcommand backs the container healthcheck.

Graceful shutdown is provided by `serve`, which listens for SIGTERM and SIGINT and drains in-flight requests. `serve` also logs the startup banner (service, version, port) from the `ServerInfo` the service passes in.

Cursor pagination for customer-facing lists (the query belongs in the entity's repository):

```rust
sqlx::query_as::<_, Row>(
    "SELECT * FROM entities WHERE is_active = TRUE \
     AND ($1::timestamptz IS NULL OR (created_at, id) < ($1, $2)) \
     ORDER BY created_at DESC, id DESC LIMIT $3",
)
.bind(cursor.map(|cursor| cursor.created_at))
.bind(cursor.map(|cursor| cursor.id))
.bind(take + 1)
```

Fetch `take + 1` rows; if more than `take` came back, drop the extra and return its predecessor as `nextCursor`.

POST endpoints creating resources accept `x-idempotency-key`. Propagate `x-correlation-id` through downstream calls for distributed tracing.

## Testing

Every service has service-layer tests in `tests/services` using `#[sqlx::test]` against a real Postgres, plus HTTP-level tests in `tests/integration` and pure tests in `tests/unit`. See `.claude/rules/testing.md`.

## Running services

```bash
cargo run --bin api-gateway
cargo run --bin customer-api
cargo run --bin admin-api
cargo run --bin schedule-api
```

or `./devops/scripts/dev.sh` to start them all. Each binary loads its own `.env` through `dotenvy`.

## Downstream calls

Calls from one service to another (gateway → services, schedule-api → webhooks) are sequential unless a task explicitly requires otherwise; use the shared `reqwest::Client` with timeouts and never block the Tokio runtime with synchronous I/O.

## Before marking complete

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ru5ty-gate-<service>
```
