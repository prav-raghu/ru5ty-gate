# Admin API

Rust service built on Axum and Tokio. Port `4001`.

Administrative API: admin authentication with MFA and password reset, the one-time admin bootstrap, user management, batch operations, reporting, and the captive portal venues, gateways and sessions. It also serves the device API the router agent talks to, and owns the database migrations (`admin-api migrate`).

## Run

```bash
cp .env.example .env
cargo run --bin admin-api
```

`admin-api healthcheck` performs a request against `/api/v1/ping` and exits 0 or 1; the container healthcheck uses it.

## Environment

`.env.example` lists every variable. Required or notable: DATABASE_URL, REDIS_URL, JWT_SECRET, JWT_REFRESH_SECRET, TWO_FACTOR_ENCRYPTION_KEY, ADMIN_WEB_URL, ADMIN_BOOTSTRAP_ENABLED, MAILTRAP_*. Configuration is loaded once into a typed `ServiceConfig` at startup and the process exits if anything is missing or invalid.

## Captive portal

Admin endpoints (admin JWT, `venue:read` to view and `venue:write` to change; only Super Admins have `venue:write`):

| Method | Path | Purpose |
|---|---|---|
| GET, POST | `/api/v1/venues` | List and create venues (`code` is the id the agent is configured with) |
| GET, PUT | `/api/v1/venues/{venueRef}` | Read and update a venue (`venueRef` is the venue UUID) |
| GET, POST | `/api/v1/venues/{venueRef}/gateways` | List gateways with their last heartbeat, and register one |
| POST | `/api/v1/gateways/{gatewayId}/rotate-key` | Issue a new API key and invalidate the old one |
| GET | `/api/v1/venues/{venueRef}/sessions` | Captive sessions, newest first; `openOnly=true` hides ended ones |

Creating a gateway or rotating its key returns `apiKey` once, in the form `<gatewayId>.<secret>`. Only a SHA-256 hash of the secret is stored, so a lost key means rotating it.

Device endpoints, called by `ru5ty-gate-agent` (see `common/central-client/README.md` for the contract). They authenticate with `Authorization: Bearer <apiKey>` instead of a JWT, and a key only works for the venue its gateway belongs to:

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/venues/{venueCode}/sessions/validate` | Grant or deny a client; returns the venue's duration and redirect |
| GET | `/api/v1/venues/{venueCode}/policy` | Venue policy the agent caches for offline use |
| POST | `/api/v1/venues/{venueCode}/heartbeat` | Records agent version, uptime, session count and queue depth on the gateway |
| POST | `/api/v1/venues/{venueCode}/sync` | Applies buffered `session_start` and `session_end` events idempotently |

Device responses are bare snake_case JSON rather than the `{ isSuccessful, data }` envelope, because that is the wire format the agent already speaks. Venues with `allowNewSessions = false` make `validate` answer `allow: false`.

## Layout

See `.claude/rules/backend.md` for the annotated directory structure: `controllers/`, `services/`, `routes/`, `schemas/`, `dtos/`, `plugins/`, `guards/`, `types/` and `tests/`.

## Test

```bash
export DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres
export TEST_REDIS_URL=redis://127.0.0.1:6379
cargo test -p ru5ty-gate-admin-api
cargo clippy -p ru5ty-gate-admin-api --all-targets -- -D warnings
```
