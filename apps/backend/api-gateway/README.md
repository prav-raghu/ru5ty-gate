# API Gateway

Rust service built on Axum and Tokio. Port `4000`.

Reverse proxy in front of the other services. Strips the `/admin` and `/scheduler` prefixes, aggregates health (`/health/services`), exposes Prometheus metrics and an optional GraphQL layer (`GRAPHQL_ENABLED`).

## Run

```bash
cp .env.example .env
cargo run --bin api-gateway
```

`api-gateway healthcheck` performs a request against `/health/live` and exits 0 or 1; the container healthcheck uses it.

## Environment

`.env.example` lists every variable. Required or notable: CUSTOMER_API_URL, ADMIN_API_URL, SCHEDULER_API_URL, RATE_LIMIT_MAX, GRAPHQL_ENABLED, GRAPHQL_PATH, GRAPHQL_PLAYGROUND, GRAPHQL_INTROSPECTION. Configuration is loaded once into a typed `ServiceConfig` at startup and the process exits if anything is missing or invalid.

## Layout

See `.claude/rules/backend.md` for the annotated directory structure: `controllers/`, `services/`, `routes/`, `schemas/`, `dtos/`, `plugins/`, `guards/`, `types/` and `tests/`.

## Test

```bash
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres
export TEST_REDIS_URL=redis://127.0.0.1:6379
cargo test -p ru5ty-gate-api-gateway
cargo clippy -p ru5ty-gate-api-gateway --all-targets -- -D warnings
```
