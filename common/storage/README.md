# ru5ty-gate-storage

S3-compatible file storage (AWS S3, Cloudflare R2).

## Public API

StorageService, S3Config, validate_upload, build_object_key

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-storage = { version = "1.0.0", path = "../../../common/storage" }
```

```rust
use ru5ty_gate_storage;
```

## Test

```bash
cargo test -p ru5ty-gate-storage
```
