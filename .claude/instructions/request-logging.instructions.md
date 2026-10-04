# Request / Response Logging — Implementation Instructions

Every Rust backend service in this template applies the shared `request_logger` middleware from `common/http`, which logs all inbound requests and outbound responses through `tracing`. In production the output is one JSON object per line on stdout (collected by Coolify/Docker); in development it is human-readable.

---

## Middleware location

```
common/http/src/middleware/request_logger.rs
```

The middleware is shared — a new service only adds `.layer(from_fn(request_logger))` in `application.rs` in the documented position. Do not copy or fork it.

---

## What it logs

| Stage | Logged fields | Skipped when |
| ----- | ------------- | ------------ |
| Request in | `method`, `url`, `correlation_id` | `OPTIONS`, paths ending `/health` or `/ready`, `/docs*` |
| Request body | masked JSON body (debug level only) | `OPTIONS`, `GET`, health paths, non-JSON bodies, `LOG_LEVEL` above debug |
| Response out | `method`, `path`, `status`, `elapsed_ms` (error level for 5xx, info otherwise) | `OPTIONS`, health paths |

Sensitive keys in a logged body are replaced with `[REDACTED]` by `mask_sensitive` in `common/logging/src/redaction.rs` (`password`, `currentPassword`, `newPassword`, `token`, `accessToken`, `refreshToken`, `secret`, `apiKey`, `clientSecret`, `privateKey`, `cardNumber`, `cvv`, `ssn`, `pin`, `otp`, `twoFactorCode`, `authorization`, `cookie`, ...). Extend `SENSITIVE_KEYS` there when a new secret-bearing field is introduced.

`elapsed_ms` is the wall-clock time from the start of the middleware to the response being produced. Use it to identify slow endpoints. Bodies are read only up to 1 MiB.

---

## Registration

Register in `application.rs` in the fixed order: `catch_panic` → `cors` → `security_headers` → `rate_limit` → `request_logger` → `api_version` (→ `response_timestamp` on admin-api and schedule-api):

```rust
let middleware = ServiceBuilder::new()
    .layer(catch_panic_layer())
    .layer(cors_layer(&self.state.config.cors_origin))
    .layer(from_fn(security_headers))
    .layer(from_fn_with_state(global_limiter, rate_limit))
    .layer(from_fn(request_logger))
    .layer(from_fn(api_version));
```

---

## Initialising logging

`main.rs` calls `init_logging("service-name")` once, after `init_sentry`. The filter comes from `RUST_LOG` when set, otherwise from `LOG_LEVEL` (`trace`, `debug`, `info`, `warn`, `error`, `silent`; default `info`). `APP_ENV=production` switches to JSON output with flattened event fields.

Log with structured fields, never string-concatenated values:

```rust
tracing::info!(user_id = %user.id, "profile updated");
tracing::warn!(%error, "token blacklist lookup failed");
```

---

## Correlation IDs

Pass `X-Correlation-ID` as a request header to tie a distributed request chain together. The middleware reads it and includes it in the incoming-request log line. The api-gateway forwards the header to downstream services; clients and other services should generate a UUID and forward it on all downstream calls:

```rust
request.header("x-correlation-id", Uuid::new_v4().to_string())
```

Find a correlated chain by filtering the collected JSON logs on `correlation_id`.

---

## Querying logs

Logs are plain JSON lines, so any collector works (Coolify log view, Loki, OpenObserve ingesting stdout):

```
{ "level": "INFO", "method": "GET", "path": "/api/v1/users", "status": 200, "elapsed_ms": 12 }
```

Filter on `status >= 500` for errors, `elapsed_ms > 500` for slow requests, `correlation_id = "..."` for a request chain.

OTLP export of logs and traces is not part of the Rust services yet; ship stdout to OpenObserve with its log agent, or add `tracing-opentelemetry` to `common/observability` when native export is required.

---

## Do not log

- Raw IP addresses — hash them with `hash_ip` from `common/logging` before logging
- Auth headers, tokens, passwords, TOTP codes — `mask_sensitive` covers bodies; never log headers wholesale
- Health/readiness endpoints — filtered by `should_skip` to avoid noise
- OPTIONS preflight requests — filtered to avoid noise

---

## Sentry runs independently of the request log

Sentry captures exceptions separately from request/response logging. When a handler returns `AppError::Internal`, `IntoResponse` logs the error at `error` level once and calls `capture_error` from `ru5ty-gate-observability`, which sends it to Sentry. Panics are captured by Sentry's panic integration and converted to a 500 by `catch_panic_layer`. They are two distinct sinks for two distinct purposes — structured telemetry on stdout, exception aggregation and alerting in Sentry. Do not collapse them into a single call or assume one replaces the other. Sentry is a no-op when `SENTRY_DSN` is unset, so local dev still logs to stdout only.
