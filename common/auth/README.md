# ru5ty-gate-auth

JWT token service: issue, verify, rotate, logout, blacklist and per-user `minIat` invalidation.

## Public API

TokenService, AuthConfig, TokenPayload

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-auth = { version = "1.0.0", path = "../../../common/auth" }
```

```rust
use ru5ty_gate_auth;
```

## Test

```bash
cargo test -p ru5ty-gate-auth
```
