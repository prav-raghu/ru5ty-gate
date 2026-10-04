# ru5ty-gate-metrics

Prometheus metrics and health check helpers.

## Public API

MetricsRegistry, track_http, health_router, HealthCheckBuilder

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-metrics = { version = "1.0.0", path = "../../../common/metrics" }
```

```rust
use ru5ty_gate_metrics;
```

## Test

```bash
cargo test -p ru5ty-gate-metrics
```
