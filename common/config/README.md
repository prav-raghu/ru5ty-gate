# ru5ty-gate-config

Environment variable access.

## Public API

EnvReader (optional, required, parse_required, parse_or), ConfigError

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-config = { version = "1.0.0", path = "../../../common/config" }
```

```rust
use ru5ty_gate_config;
```

## Test

```bash
cargo test -p ru5ty-gate-config
```
