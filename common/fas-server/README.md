# ru5ty-gate-fas-server

Forwarding Authentication Service (FAS) HTTP server for openNDS.

openNDS intercepts an unauthenticated client and redirects the browser to this server with the client and gateway described in the query string. The server decides grant or deny, consulting the central platform and falling back to local policy if it is unreachable. On grant it redirects the browser back to openNDS's own auth endpoint, which installs the firewall rule that lets the client's traffic through.

Reference: [openNDS FAS documentation](https://opennds.readthedocs.io/en/latest/fas.html).

## Public API

router, AppState, FasConfig, FasQuery

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Routes

| Method | Path | Purpose |
|---|---|---|
| GET | `/fas` | openNDS auth callout |
| GET | `/health` | Liveness probe for the agent process |
| GET | `/status` | Active session count and number of sync events still buffered |

## Notes

- `FasQuery` carries the parameters openNDS appends. `authaction`, `gatewaymac` and `clientif` are optional because they depend on the `fas_secure_enabled` level and the gateway interface setup.
- `hid` is openNDS's per-attempt token. It must be echoed back verbatim as `tok` so openNDS can match the callback to the client it is holding captive.
- `FasQuery::auth_redirect_url` prefers the `authaction` URL when openNDS supplies one, and otherwise builds `http://<gatewayaddress>:<gatewayport>/opennds_auth/`.
- `FasConfig` is the subset of agent settings this crate needs. The agent maps `Settings` into it, so this crate does not depend on the config crate.
- When `FasConfig::gateway_name` is set, requests carrying a different `gatewayname` get a 400. v1 is single-gateway by design.
- Offline tolerance: an outage on the central-platform side is never itself the reason a client is denied, unless the operator explicitly sets `allow_offline = false`.

## Use

```toml
ru5ty-gate-fas-server = { version = "1.0.0", path = "../../../common/fas-server" }
```

```rust
use ru5ty_gate_fas_server::router;
```

## Test

```bash
cargo test -p ru5ty-gate-fas-server
```
