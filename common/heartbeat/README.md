# ru5ty-gate-heartbeat

Scheduled background tasks for the captive portal agent: a periodic device heartbeat, and batched sync of buffered session events.

Both tasks are independent of each other and of the FAS request path, so a slow or unreachable central platform never blocks a client's captive-portal auth.

## Public API

spawn_heartbeat_task, spawn_sync_task

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Behaviour

- Heartbeat: every interval, post uptime and active session count to the central platform. The first tick is skipped so no heartbeat is sent before the server has finished starting. A failed post is logged and retried on the next tick. There is no backoff or buffering, because a heartbeat is a latest-state signal rather than an event log.
- Sync: every interval, read up to `batch_size` pending events from the session store and push them to the central platform. Events stay queued, and are retried, until the central platform acknowledges them. This is what lets the local buffer survive an outage without losing session history.

## Use

```toml
ru5ty-gate-heartbeat = { version = "1.0.0", path = "../../../common/heartbeat" }
```

```rust
use ru5ty_gate_heartbeat::spawn_sync_task;
```

## Test

```bash
cargo test -p ru5ty-gate-heartbeat
```
