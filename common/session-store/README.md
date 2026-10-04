# ru5ty-gate-session-store

Local, durable session store for the captive portal agent.

Active client sessions (MAC, token, expiry, venue) live in a local sqlite file, so the agent survives restarts and central-platform outages. Session lifecycle events are queued in the same file for batched sync once the central API is reachable again. This is the same offline-buffer pattern used in the C++ water project, kept consistent on purpose.

## Public API

SessionStore (open, open_in_memory, upsert_session, get_session, sweep_expired, end_session, rebase_untrusted_sessions, delete_expired, active_session_count, enqueue_event, pending_events, mark_synced, pending_event_count, prune_synced, cap_pending_events, set_meta, get_meta, save_policy, load_policy), Session, StoredPolicy, SyncEvent, SyncEventKind, StoreError, CURRENT_SCHEMA_VERSION, unix_now, build_unix, clock_is_trusted

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Notes

- `SessionStore` is cheap to clone: clones share one connection behind `Arc<Mutex<_>>`. Every operation hops onto the blocking thread pool through `spawn_blocking`, because rusqlite is synchronous.
- `SyncEventKind` only covers session start and end. Heartbeats are posted directly and never pass through the queue.
- `sweep_expired` and `end_session` delete the session and queue its `session_end` event in one transaction, so a session cannot disappear without its event.
- MAC addresses are stored lower case and matched case-insensitively.
- `prune_synced` and `cap_pending_events` bound the sqlite file: synced events are deleted after a retention period and the oldest pending events are dropped beyond a cap.
- Sessions granted while `clock_is_trusted` was false are flagged. `sweep_expired` ignores them and `rebase_untrusted_sessions` restarts their full duration once the clock is valid. `build_unix` is the binary's build time, embedded by `build.rs`.
- `save_policy` and `load_policy` cache the central policy in the `meta` table so it applies while the central platform is down.
- `CURRENT_SCHEMA_VERSION` is bumped whenever the schema changes in a way that needs more than `CREATE TABLE IF NOT EXISTS`. Version 2 added the `clock_untrusted` column and the `meta` table; opening a version 1 database adds the column in place.
- `rusqlite` is pinned to the 0.39 line on purpose. `sqlx` (used by the Postgres services in this workspace) resolves `libsqlite3-sys` 0.37, and Cargo allows only one crate in the graph to link the native sqlite library. Moving `rusqlite` to a release that needs a newer `libsqlite3-sys` fails dependency resolution until `sqlx` catches up.

## Use

```toml
ru5ty-gate-session-store = { version = "1.0.0", path = "../../../common/session-store" }
```

```rust
use ru5ty_gate_session_store::SessionStore;
```

## Test

```bash
cargo test -p ru5ty-gate-session-store
```
