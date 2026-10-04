# ru5ty-gate-agent-config

TOML settings for the captive portal agent.

Config is a single TOML file. v1 is single-venue and single-gateway, so the layout is flat: no per-venue arrays and no advertiser walled-garden settings. Those are deferred to the full Gosling spec if v1 proves out.

## Public API

Settings (parse, load_from, load), VenueSettings, ServerSettings, CentralSettings, SessionSettings, HeartbeatSettings, SyncSettings, ConfigError, CONFIG_PATH_ENV, DEFAULT_CONFIG_PATH

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Resolution

`Settings::load` reads the file named by `RU5TY_GATE_CONFIG`, falling back to `config/agent.toml`. `Settings::load_from` reads an explicit path. Every section except `venue` and `server` has defaults, so a minimal file only needs a venue id, a bind address and a database path.

## Notes

- `venue.gateway_name`: the openNDS `gatewayname` the agent expects on inbound FAS requests. When set, requests for a different gateway are rejected.
- `central.timeout_secs`: kept short so an unreachable central platform never stalls the FAS auth round-trip. The agent falls back to local policy instead.
- `session.allow_offline`: grant clients from local policy when the central API is unreachable (fail open), or deny them (fail closed).
- This crate deliberately uses TOML rather than the `EnvReader` environment pattern in `ru5ty-gate-config`, because the agent runs on an OpenWrt router where a config file is the natural interface.

## Use

```toml
ru5ty-gate-agent-config = { version = "1.0.0", path = "../../../common/agent-config" }
```

```rust
use ru5ty_gate_agent_config::Settings;
```

## Test

```bash
cargo test -p ru5ty-gate-agent-config
```
