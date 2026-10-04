# ru5ty-gate-observability

Sentry initialisation and error capture.

## Public API

init_sentry, capture_error

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-observability = { version = "1.0.0", path = "../../../common/observability" }
```

```rust
use ru5ty_gate_observability;
```

## Test

```bash
cargo test -p ru5ty-gate-observability
```
