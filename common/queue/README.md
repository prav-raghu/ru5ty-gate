# ru5ty-gate-queue

Redis sorted-set job queue and worker.

## Public API

QueueService, WorkerService, JobHandler, QueueJobOptions

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-queue = { version = "1.0.0", path = "../../../common/queue" }
```

```rust
use ru5ty_gate_queue;
```

## Test

```bash
cargo test -p ru5ty-gate-queue
```
