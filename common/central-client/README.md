# ru5ty-gate-central-client

Async HTTP client for the central platform API.

Validates new sessions, pulls policy, posts heartbeats and syncs buffered session events in batches. Every call is expected to fail sometimes: the FAS decision path and the background tasks treat an unreachable central platform as a normal, handled case rather than an error to propagate.

## Public API

CentralClient (new, validate_session, fetch_policy, post_heartbeat, sync_events), ClientConfig, ValidateSessionRequest, ValidateSessionResponse, PolicyResponse, HeartbeatRequest, SyncEventDto, SyncBatchRequest, SyncBatchResponse, CentralError

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Endpoints

All paths are relative to `{base_url}/v1/venues/{venue_id}`:

| Method | Path | Purpose |
|---|---|---|
| POST | `/sessions/validate` | Ask whether a client session should be granted, and with what duration and redirect |
| GET | `/policy` | Pull the venue policy (session duration, redirect URL) |
| POST | `/heartbeat` | Device health check-in |
| POST | `/sync` | Push a batch of buffered session events |

When `api_key` is configured it is sent as a bearer token on every request.

## Notes

`ClientConfig` is built by the agent from `ru5ty-gate-agent-config::Settings`. It stays separate here so this crate does not depend on the config crate.

## Use

```toml
ru5ty-gate-central-client = { version = "1.0.0", path = "../../../common/central-client" }
```

```rust
use ru5ty_gate_central_client::CentralClient;
```

## Test

```bash
cargo test -p ru5ty-gate-central-client
```
