# ru5ty-gate-session-store

Local, durable session store for the captive portal agent.

Active client sessions (MAC, token, expiry, venue) live in a local sqlite file, so the agent survives restarts and central-platform outages. Session lifecycle events are queued in the same file for batched sync once the central API is reachable again. This is the same offline-buffer pattern used in the C++ water project, kept consistent on purpose.

## Public API

SessionStore (open, open_in_memory, upsert_session, get_session, delete_expired, active_session_count, enqueue_event, pending_events, mark_synced, pending_event_count), Session, SyncEvent, SyncEventKind, StoreError, CURRENT_SCHEMA_VERSION, unix_now

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Notes

- `SessionStore` is cheap to clone: clones share one connection behind `Arc<Mutex<_>>`. Every operation hops onto the blocking thread pool through `spawn_blocking`, because rusqlite is synchronous.
- `SyncEventKind` only covers session start and end. Heartbeats are posted directly and never pass through the queue.
- `CURRENT_SCHEMA_VERSION` is bumped whenever the schema changes in a way that needs more than `CREATE TABLE IF NOT EXISTS`. v1 has no migration steps yet; the constant exists so a future breaking change has somewhere to hook in.
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
