# ru5ty-gate-utilities

Crypto, date, password and webhook signature helpers.

## Public API

CryptoUtil, DateUtil, PasswordUtil, WebhookSignatureService, ApiVersionManager

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-utilities = { version = "1.0.0", path = "../../../common/utilities" }
```

```rust
use ru5ty_gate_utilities;
```

## Test

```bash
cargo test -p ru5ty-gate-utilities
```
