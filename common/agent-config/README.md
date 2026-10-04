# ru5ty-gate-agent-config

TOML settings for the captive portal agent.

Config is a single TOML file. v1 is single-venue and single-gateway, so the layout is flat: no per-venue arrays and no advertiser walled-garden settings. Those are deferred to the full Gosling spec if v1 proves out.

## Public API

Settings (parse, load_from, load, validate), VenueSettings, ServerSettings, FasSettings, CentralSettings, SessionSettings, HeartbeatSettings, SyncSettings, PrivacySettings, LimitsSettings, EnforcementSettings, ConfigError, CONFIG_PATH_ENV, DEFAULT_CONFIG_PATH

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Resolution

`Settings::load` reads the file named by `RU5TY_GATE_CONFIG`, falling back to `config/agent.toml`. `Settings::load_from` reads an explicit path. `venue`, `server` and `fas` are required. Every other section has defaults, so a minimal file needs a venue id, the bind and database paths, a `faskey` and either a `privacy.hash_pepper` or `privacy.send_raw_identifiers = true`.

`Settings::parse` validates the result, so an invalid file never produces a `Settings`. Rules: `venue.id` is 1 to 64 letters, digits, `-` or `_`; both bind addresses parse as `ip:port` and differ; every interval, timeout and limit is at least 1; `sync.batch_size` is 1 to 1000 and `sync.max_pending` is at least the batch size; `fas.faskey`, `central.api_key` and `privacy.hash_pepper` are at least 16 characters; `central.base_url` is http or https, and `https` is required when an API key is set unless the host is loopback or `central.allow_insecure_http` is true.

## Notes

- `venue.gateway_name`: the openNDS `gatewayname` the agent expects on inbound FAS requests. When set, requests for a different gateway are rejected.
- `central.timeout_secs`: kept short so an unreachable central platform never stalls the FAS auth round-trip. The agent falls back to local policy instead.
- `fas.faskey`, `central.api_key` and `privacy.hash_pepper` are `SecretString` values: they are redacted in `Debug` output and must be read with `expose_secret()`.
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
