# ru5ty-gate-central-client

Async HTTP client for the central platform API.

Validates new sessions, pulls policy, posts heartbeats and syncs buffered session events in batches. Every call is expected to fail sometimes: the FAS decision path and the background tasks treat an unreachable central platform as a normal, handled case rather than an error to propagate.

## Public API

CentralClient (new, validate_session, fetch_policy, post_heartbeat, sync_events), ClientConfig, IdentifierPolicy, ValidateSessionRequest, ValidateSessionResponse, PolicyResponse, HeartbeatRequest, SyncEventDto, SyncBatchRequest, SyncBatchResponse, CentralError

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Endpoints

All paths are relative to `{base_url}/v1/venues/{venue_id}`, where `venue_id` is the venue `code` configured in the agent. The central platform implements them in `admin-api` (see its README), so `base_url` is the admin API address plus `/api`, for example `https://central.example.com/admin/api` when going through the gateway:

| Method | Path | Purpose |
|---|---|---|
| POST | `/sessions/validate` | Ask whether a client session should be granted, and with what duration and redirect |
| GET | `/policy` | Pull the venue policy (session duration, redirect URL) |
| POST | `/heartbeat` | Device health check-in |
| POST | `/sync` | Push a batch of buffered session events |

When `api_key` is configured it is sent as a bearer token on every request. Gateways receive their key from the admin API in the form `<gatewayId>.<secret>`; a key only works for its own venue.

Bodies and responses are bare snake_case JSON without the `{ isSuccessful, data }` envelope the rest of the platform uses, and request bodies reject unknown fields. Change the DTOs here and in `admin-api`'s `device_schema.rs` together. `heartbeat` answers `204`.

## Notes

`ClientConfig::api_key` is a `SecretString`, so the key never shows up in `Debug` output.

`IdentifierPolicy` decides what the central platform learns about clients. `raw()` passes MAC and IP addresses through. `hashed(pepper)` replaces them with salted SHA-256 hashes (`mac_hash`, `client_ip_hash` in event payloads) and `log_mac` gives a short hash or masked MAC that is safe to log; raw MACs are never logged.

`HeartbeatRequest` carries `pending_events` and `last_sync_ok_at` so operators can see a stuck queue from the dashboard.

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
