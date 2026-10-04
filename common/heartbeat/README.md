# ru5ty-gate-heartbeat

Scheduled background tasks for the captive portal agent: a periodic device heartbeat, batched sync of buffered session events, and a cache refresh of the central policy.

All tasks are independent of each other and of the FAS request path, so a slow or unreachable central platform never blocks a client's captive-portal auth.

## Public API

spawn_heartbeat_task, spawn_sync_task, spawn_policy_task, HeartbeatTaskConfig, SyncTaskConfig, SyncStatus

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Behaviour

- Heartbeat: every interval, post uptime, active sessions, queue depth and the time of the last successful sync (from `SyncStatus`). The first tick is skipped so no heartbeat is sent before the server has finished starting. A failed post is logged and retried on the next tick; a heartbeat is a latest-state signal, so there is no backoff or buffering.
- Sync: every interval, read up to `batch_size` pending events and push them. Events stay queued, and are retried, until the central platform acknowledges them. Before each batch the task drops the oldest pending events beyond `max_pending` and deletes synced events older than `retain_synced_secs`. Event payloads pass through `IdentifierPolicy`, so hashed identifiers replace raw MAC and IP addresses unless the operator opted in to raw ones.
- Policy: every interval, fetch the venue policy and cache it in the session store, so the agent keeps applying it while the central platform is down.

Intervals must be at least one second; `ru5ty-gate-agent-config` validates this before any task starts.

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
