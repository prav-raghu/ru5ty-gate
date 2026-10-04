# ru5ty-gate-webhooks

Shared webhook delivery service with retry policy and SSRF protection.

## Public API

WebhookDeliveryService, TargetPolicy, next_retry_delay_seconds

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-webhooks = { version = "1.0.0", path = "../../../common/webhooks" }
```

```rust
use ru5ty_gate_webhooks;
```

## Test

```bash
cargo test -p ru5ty-gate-webhooks
```
