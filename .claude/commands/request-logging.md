# Request / Response Logging

All backend services use a centralised `request_logger` middleware from `common/http` for request and response logging. Controllers never log their own request/response data — the middleware handles it at the framework level. The full reference is `.claude/instructions/request-logging.instructions.md`; this command is the short checklist for wiring it into a service.

## How It Works

```
Request arrives
     │
     ▼
request_logger → logs: method, url, x-correlation-id        (skips OPTIONS, /health, /ready, /docs)
     │
     ▼
  (body is read only when LOG_LEVEL is debug and the method is not GET)
     │
     ▼
                       logs: masked request body             (debug level only)
     │
     ▼
  (handler runs)
     │
     ▼
response       → logs: method, path, status, elapsed_ms      (error level for 5xx)
```

## Sensitive Field Masking

Before any body is written to the log it passes through `mask_sensitive` in `common/logging/src/redaction.rs`. It deep-walks the JSON value and replaces the value of every key in `SENSITIVE_KEYS` with `[REDACTED]`.

| Category | Fields |
|----------|--------|
| Passwords | `password`, `currentPassword`, `newPassword`, `confirmPassword` |
| Tokens | `token`, `accessToken`, `refreshToken` |
| Secrets | `secret`, `apiKey`, `api_key`, `clientSecret`, `privateKey` |
| Financial | `creditCard`, `cardNumber`, `cvv`, `cvc` |
| Identity | `ssn`, `nationalId`, `pin` |
| MFA | `otp`, `twoFactorCode` |
| Headers | `authorization`, `cookie` |

Add new secret-bearing field names to `SENSITIVE_KEYS` in the same change that introduces them.

## Wiring It Into a Service

1. Add `request_logger` to the layer stack in `application.rs`, after `rate_limit` and before `api_version`:

```rust
let middleware = ServiceBuilder::new()
    .layer(catch_panic_layer())
    .layer(cors_layer(&self.state.config.cors_origin))
    .layer(from_fn(security_headers))
    .layer(from_fn_with_state(global_limiter, rate_limit))
    .layer(from_fn(request_logger))
    .layer(from_fn(api_version));
```

2. Call `init_logging("<service-name>")` in `main.rs` after `init_sentry`.
3. Log inside services with structured `tracing` fields — never `println!`, never string-built messages with secrets.
4. Do not copy the middleware into the service; it is shared.

## Correlation IDs

Forward `X-Correlation-ID` on every downstream call so a request chain can be followed through the logs.

## Testing

`common/http/tests` covers the middleware. When adding a new sensitive field, add an assertion to the redaction test in `common/logging/tests`.
