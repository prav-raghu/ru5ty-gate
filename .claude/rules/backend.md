---
paths:
  - "apps/backend/**/*.rs"
  - "apps/backend/**/Cargo.toml"
---

# Backend Service Rules

You are working on a Rust backend service built on Axum 0.8 and Tokio. These rules apply to all files under `apps/backend/`.

## Non-negotiable

- Axum on Tokio only — no Actix, Rocket, Warp
- `validator` derives on request structs for ALL backend validation, with `#[serde(deny_unknown_fields)]` — never hand-rolled validation in controllers
- No `unsafe` (forbidden by the workspace lint) and no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!` outside tests (denied by clippy)
- No comments in code
- No hardcoded secrets — every secret comes from the environment
- DTOs, schemas and models are plain structs/enums, one item per file
- Controllers are unit structs whose async associated functions take `State<AppState>` and extractors; services are structs with `impl` blocks and constructor-injected dependencies

## Middleware order (must not change)

`catch_panic` → `cors` → `security_headers` → `rate_limit` → `request_logger` → `api_version` → (`response_timestamp` for admin-api and schedule-api)

Layers are composed with `tower::ServiceBuilder` in `application.rs`, listed top to bottom in execution order. The first layer of every service sets the `TrustedProxyHops` extension from `TRUSTED_PROXY_HOPS` (default `1`), which `ClientIp` needs to pick the real client out of `X-Forwarded-For`; never read that header directly. Routes register after the layers are defined: `Router::new().nest("/api/v1", V1Routes::register(&state))`.

## Directory structure (immutable)

```
apps/backend/[service-name]/
├── Cargo.toml
├── Dockerfile
├── .env
├── .env.example
├── README.md
├── src/
│   ├── main.rs                  binary entry: healthcheck subcommand, env, logging, serve
│   ├── lib.rs                   module declarations and re-exports
│   ├── application.rs           Application { state }, router(), start()
│   ├── config/                  service_config.rs, rate_limit_config.rs, mod.rs
│   ├── controllers/             one {entity}_controller.rs per entity
│   ├── services/                one {entity}_service.rs per entity
│   ├── routes/
│   │   ├── {entity}_route.rs    public()/protected() routers per entity
│   │   ├── v1/                  v1_route.rs composes public + protected routers
│   │   └── v2/
│   ├── schemas/                 {entity}_schema.rs request structs with validator derives
│   ├── dtos/                    {entity}_dto.rs response structs
│   ├── plugins/                 database.rs, services.rs, auth_guard.rs (builders)
│   ├── guards/                  auth_guard.rs
│   └── types/                   app_state.rs, services.rs
└── tests/
    ├── common/mod.rs            shared helpers, factories, recording email sender
    ├── services/                service-layer tests (required)
    ├── integration/             full HTTP tests via tower::ServiceExt::oneshot
    └── unit/                    config and validation tests
```

Each folder has a `mod.rs` that declares the files and re-exports the public items. Every struct, enum, trait and constant lives in its own file.

## Environment configuration

Every service builds a typed `ServiceConfig` once at startup using `EnvReader` from `ru5ty-gate-config`. Never call `std::env::var` outside `main.rs` and `EnvReader::from_process`. Required values use `required`/`parse_required`; defaults use `parse_or`. Invalid or missing values return `ConfigError` and the process exits before binding a port. See `env-config.instructions.md`.

`APP_ENV` selects `development` or `production`. `DATABASE_URL` and `REDIS_URL` are the only connection settings.

`CORS_ORIGIN` is one origin or a comma-separated list (`http://localhost:4005,http://localhost:4007`). Use a list when an API is called from more than one app. A value that matches no request origin allows nothing.

`.env` files are parsed by `dotenvy` (the services at startup, and CodeLLDB for the F5 launch configs). Quote any value that contains spaces, for example `MAILTRAP_FROM_NAME="Your App Name"`. An unquoted value is a parse error; the services ignore it and stop loading at that line, so every variable below it is silently missing, and F5 fails with `Error parsing line`. Use `%24` for a literal `$` in a URL password.

## AppState and service container

`AppState { config: Arc<ServiceConfig>, pool: PgPool, redis: RedisService, services: Arc<Services> }` is cloned into every handler through `State`. `plugins/services.rs::build_services` instantiates every service once. No DI framework — direct construction.

## Dates — ISO 8601 on the wire, always

Request and response dates are ISO 8601 in both directions via `chrono::DateTime<Utc>` serialised by serde — never `dd/MM/yyyy` on the backend, that is a UI-only display format. `DateUtil` in `ru5ty-gate-utilities` covers parsing and validation. See `date-handling.instructions.md`.

## Response envelope (always)

```rust
ApiResponse<T> { is_successful: bool, data: Option<T>, message: Option<String>, errors: Option<Vec<FieldError>>, date_time_stamp: Option<String> }
```

Serialised in camelCase: `{ isSuccessful, data, message, errors, dateTimeStamp }`. Handlers return `Result<Response, AppError>`; `AppError` renders the envelope and status code. `date_time_stamp` is added by `response_timestamp` on admin-api and schedule-api only.

## Validation error format (400 responses)

Request bodies go through `ValidatedJson<T>`, queries through `ApiQuery<T>` and paths through `ApiPath<T>`. Failures return a field-level 400:

```json
{
  "isSuccessful": false,
  "message": "Validation failed",
  "errors": [
    { "field": "email", "message": "Must be a valid email address" },
    { "field": "name", "message": "Name is required" }
  ]
}
```

## Error status contract

| Scenario | Status | `AppError` variant |
|---|---|---|
| Successful create | 201 | — |
| Successful read/update/delete | 200 | — |
| Validation failure | 400 | `Validation` |
| Business rule violation | 400 | `BadRequest` |
| Auth missing or invalid | 401 | `Unauthorized` |
| Insufficient permissions | 403 | `Forbidden` |
| Not found | 404 | `NotFound` |
| Unique constraint / duplicate | 409 | `Conflict` |
| Too many requests | 429 | `TooManyRequests` |
| Unexpected error | 500 | `Internal` |

## Request schema requirements

- `#[serde(deny_unknown_fields, rename_all = "camelCase")]` on every request body — no exceptions
- `#[validate(length(min = 1))]` on every required string — an empty string deserialises successfully without it
- `#[validate(email)]` on email fields, `Uuid` field types for identifiers
- `max` length must match the `VARCHAR(N)` in the migration
- See `validation-chain.instructions.md` for the SQL → validator mapping

## SQL access

SQL lives in the repository layer in `common/database` (`src/repositories/`), not inline in services. A service builds the request, calls a repository, and maps the returned model or `*Record` to a DTO with `From`. See `documentation/repository-layer.md`.

- Plain CRUD on a table whose model implements `Entity` uses the generic `Repository::<Model>`: `find_by_id`, `list`, `count`, `exists`, `insert`, `update_by_id`, `delete_by_id`, and `deactivate` for `SoftDeletable` models
- Joins, filters, ownership checks and any other table-specific query go in a `{Entity}Repository` unit struct whose associated functions are generic over `sqlx::PgExecutor`. Row shapes are `*Record` structs in `records/`; inputs are structs in `inputs/`
- Repositories return `DatabaseError`. Services convert it with `AppError::internal` and use `DatabaseError::is_unique_violation()` to return `AppError::Conflict`
- A transaction is passed as the executor (`&mut *transaction`), so the same repository call works on the pool or inside a transaction
- Bound parameters only. Never interpolate user input into SQL; dynamic filters use `QueryBuilder` with `push_bind`. Table names come from `Entity::TABLE`, never from input
- Inline SQL that still exists in services predates the layer: admin-api (user, auth, bootstrap, reporting and batch services), customer-api (auth service and the rest of the webhook subscription service) and the webhook delivery service. Move a service's queries into repositories when you change that service. New code uses repositories from the start

## Health, readiness and metrics

Every service exposes public `GET /health` (and `/ready` where it has a database) under `HealthRoutes::public()`, both excluded from request logging. The binary also supports `<binary> healthcheck` for container healthchecks. Metrics are exposed by `ru5ty-gate-metrics`.

## Graceful shutdown

`main.rs` calls `serve`, which handles SIGTERM and SIGINT through `shutdown_signal` and drains in-flight requests. Do not add a second signal handler.

## Startup banner, ports and docs link

`Application::start` builds `ServerInfo::new("<service>", env!("CARGO_PKG_VERSION"), config.port, config.production)` and passes it to `serve(router, &info)`. In development `serve` logs `<service> v<version> running on http://localhost:<port> (development)`. In production it logs one structured JSON line with `service`, `version`, `environment` and `port`.

When a service mounts Swagger UI, call `.with_docs("/docs")` on its `ServerInfo`. Development then also logs `API docs: http://localhost:<port>/docs`, and `ServerInfo::docs_url()` returns `None` in production so the link is never advertised there. No service mounts docs yet (see `openapi.instructions.md`).

Every app uses one fixed port in the 4000 range in development. The table in `CLAUDE.md` is the registry, and a new app takes the next free number.

## Rate limiting tiers

| Policy | Limit | Applies to |
|---|---|---|
| `global` | 200 req/min per client | Every route, applied as middleware |
| `auth` | 10 req/min per client | Login, register, refresh and other credential endpoints |
| `sensitive_endpoints` | 5 req/min per client | Password reset, MFA verification and similar |

Policies are built in `config/rate_limit_config.rs` as `RateLimitPolicy::per_minute(limit, message)` and enforced by `RateLimiter`. The api-gateway limit is configurable through `RATE_LIMIT_MAX`.

## Before marking complete

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ru5ty-gate-<service>
```

Zero warnings and zero failures required.
