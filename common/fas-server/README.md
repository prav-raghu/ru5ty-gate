# ru5ty-gate-fas-server

Forwarding Authentication Service (FAS) HTTP server for openNDS, speaking `fas_secure_enabled '1'`.

openNDS intercepts an unauthenticated client and redirects the browser here with one base64 `fas` value describing the client and gateway. The server validates it, decides grant or deny (consulting the central platform and falling back to local policy), and on grant redirects the browser to openNDS's auth endpoint with the token `sha256_hex(hid + faskey)`.

Reference: [openNDS FAS documentation](https://opennds.readthedocs.io/en/latest/fas.html).

## Public API

public_router, admin_router, AppState, FasConfig, RouterLimits, RateLimiter, FasRequest, FasPayload, FasPayloadDecoder, FasToken, FasError

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Routes

| Router | Method | Path | Purpose |
|---|---|---|---|
| public | GET | `/fas` | openNDS auth callout |
| public | GET | `/health` | liveness probe |
| admin | GET | `/status` | active sessions, queue depth, clock trust |
| admin | POST | `/binauth` | openNDS BinAuth deauthentication report |

Serve the public router with `into_make_service_with_connect_info::<SocketAddr>()` so the peer check can see the client's address, and bind the admin router to loopback only.

## Notes

- `FasPayloadDecoder` splits the decoded value on `", "` and each pair on the first `=`, keeps the first occurrence of a repeated field, and restores `+` characters that arrived as spaces. Fields are checked with `validator` rules; an `originurl` that is not an http(s) URL is dropped rather than rejected.
- Only level 1 is supported. A request without a `fas` value, such as the old flat level 0 query, gets `400`.
- `FasConfig::verify_client_ip` requires the TCP peer to equal the payload's `clientip`. Disable it only behind a reverse proxy.
- The public router adds a request timeout, a concurrency limit, a panic catcher, `Cache-Control: no-store` and a per-client-IP rate limit on `/fas` (`RouterLimits`).
- `IdentifierPolicy` (from `ru5ty-gate-central-client`) decides whether the central platform receives raw or hashed MAC and IP addresses, and keeps raw MACs out of log lines.
- When `FasConfig::gateway_name` is set, requests carrying a different `gatewayname` get a 400. v1 is single-gateway by design.
- Offline tolerance: an outage on the central-platform side is never itself the reason a client is denied, unless the operator explicitly sets `allow_offline = false`.

## Use

```toml
ru5ty-gate-fas-server = { version = "1.0.0", path = "../../../common/fas-server" }
```

```rust
use ru5ty_gate_fas_server::public_router;
```

## Test

```bash
cargo test -p ru5ty-gate-fas-server
```
