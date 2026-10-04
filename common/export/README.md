# ru5ty-gate-export

CSV and Excel exporters.

## Public API

CsvExporter, ExcelExporter, ExportRow

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-export = { version = "1.0.0", path = "../../../common/export" }
```

```rust
use ru5ty_gate_export;
```

## Test

```bash
cargo test -p ru5ty-gate-export
```
