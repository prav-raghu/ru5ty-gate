# ru5ty-gate-cache

Redis cache with a cloneable connection manager that can be disabled.

## Public API

RedisService (get, set_ex, del, incr, get_json, set_json, keys_matching)

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-cache = { version = "1.0.0", path = "../../../common/cache" }
```

```rust
use ru5ty_gate_cache;
```

## Test

```bash
cargo test -p ru5ty-gate-cache
```
