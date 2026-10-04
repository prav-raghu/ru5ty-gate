# Customer API

Rust service built on Axum and Tokio. Port `4002`.

Customer-facing API: registration and login, users, webhook subscriptions, user export. Routes live under `/api/v1` and `/api/v2`.

## Run

```bash
cp .env.example .env
cargo run --bin customer-api
```

`customer-api healthcheck` performs a request against `/api/v1/ping` and exits 0 or 1; the container healthcheck uses it.

## Environment

`.env.example` lists every variable. Required or notable: DATABASE_URL, REDIS_URL, JWT_SECRET, JWT_REFRESH_SECRET, CORS_ORIGIN, CUSTOMER_WEB_URL, MAILTRAP_*. Configuration is loaded once into a typed `ServiceConfig` at startup and the process exits if anything is missing or invalid.

## Layout

See `.claude/rules/backend.md` for the annotated directory structure: `controllers/`, `services/`, `routes/`, `schemas/`, `dtos/`, `plugins/`, `guards/`, `types/` and `tests/`.

## Test

```bash
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres
export TEST_REDIS_URL=redis://127.0.0.1:6379
cargo test -p ru5ty-gate-customer-api
cargo clippy -p ru5ty-gate-customer-api --all-targets -- -D warnings
```
