# Admin API

Rust service built on Axum and Tokio. Port `4001`.

Administrative API: admin authentication with MFA and password reset, the one-time admin bootstrap, user management, batch operations and reporting. It also owns the database migrations (`admin-api migrate`).

## Run

```bash
cp .env.example .env
cargo run --bin admin-api
```

`admin-api healthcheck` performs a request against `/api/v1/ping` and exits 0 or 1; the container healthcheck uses it.

## Environment

`.env.example` lists every variable. Required or notable: DATABASE_URL, REDIS_URL, JWT_SECRET, JWT_REFRESH_SECRET, TWO_FACTOR_ENCRYPTION_KEY, ADMIN_WEB_URL, ADMIN_BOOTSTRAP_ENABLED, MAILTRAP_*. Configuration is loaded once into a typed `ServiceConfig` at startup and the process exits if anything is missing or invalid.

## Layout

See `.claude/rules/backend.md` for the annotated directory structure: `controllers/`, `services/`, `routes/`, `schemas/`, `dtos/`, `plugins/`, `guards/`, `types/` and `tests/`.

## Test

```bash
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres
export TEST_REDIS_URL=redis://127.0.0.1:6379
cargo test -p ru5ty-gate-admin-api
cargo clippy -p ru5ty-gate-admin-api --all-targets -- -D warnings
```
