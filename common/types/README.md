# ru5ty-gate-types

Shared enums, DTOs and RBAC.

## Public API

RoleName, Permission, get_permissions_for_role, ApiResponse, FieldError, webhook, batch and report types

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-types = { version = "1.0.0", path = "../../../common/types" }
```

```rust
use ru5ty_gate_types;
```

## Test

```bash
cargo test -p ru5ty-gate-types
```
