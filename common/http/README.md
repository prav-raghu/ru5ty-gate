# ru5ty-gate-http

Axum building blocks shared by every service.

## Public API

AppError, ValidatedJson, ApiQuery, ApiPath, middleware (CORS, security headers, rate limit, request logger, auth, API version), serve, ServerInfo, run_healthcheck

`cors_layer` accepts one origin or a comma-separated list. `serve(router, &ServerInfo)` logs the startup banner (service, version, port) and, in development only, the API docs link when the `ServerInfo` has a docs path.

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-http = { version = "1.0.0", path = "../../../common/http" }
```

```rust
use ru5ty_gate_http;
```

## Test

```bash
cargo test -p ru5ty-gate-http
```
