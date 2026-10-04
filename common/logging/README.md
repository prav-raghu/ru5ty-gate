# ru5ty-gate-logging

Structured logging over tracing with redaction helpers.

## Public API

init_logging, mask_sensitive, hash_ip

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-logging = { version = "1.0.0", path = "../../../common/logging" }
```

```rust
use ru5ty_gate_logging;
```

## Test

```bash
cargo test -p ru5ty-gate-logging
```
