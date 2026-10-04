# Agent

Router-side daemon that runs alongside [openNDS](https://opennds.readthedocs.io/) on captive-portal hardware, targeted at a GL-iNet GL-MT6000 running OpenWrt. Dev port `4009`; on the router it listens on `2080`.

openNDS handles the captive redirect and the splash page. This agent sits behind it as the Forwarding Authentication Service (FAS): openNDS calls out to it to approve or deny a client, and it handles session persistence, heartbeat reporting and buffered sync to the central platform.

This is a Rust take on the same protocol surface as the Go agent (Gosling Systems spec): a different runtime doing the same job.

## v1 scope

- Single venue and single gateway.
- Full FAS auth round-trip with openNDS. This is the part worth proving out first; everything else is scaffolding around it.
- A local sqlite session store that survives restarts and central-API outages, with buffered session events synced to the central platform once it is reachable again.
- A basic heartbeat (uptime and active session count) posted to the central API on an interval.

Out of scope for v1: zero-PII consent capture and splash page content (that is openNDS's job), multi-venue config, and advertiser walled-garden sync. Those are deferred to the full Gosling spec if this proves out.

## Protocol: how the FAS round-trip works

1. A client on the gateway's network tries to reach the internet. openNDS intercepts it and redirects the client's browser to this agent's `GET /fas` endpoint. The query string describes the client and gateway: `clientip`, `clientmac`, `gatewayname`, `gatewayaddress`, `gatewayport`, `originurl`, `hid`, and optionally `authaction`, `gatewaymac` and `clientif` depending on openNDS's `fas_secure_enabled` level.
2. The agent asks the central platform whether to grant this client (`ru5ty-gate-central-client::validate_session`) with a short timeout. If the central platform is unreachable it falls back to local policy; `session.allow_offline` in the config selects fail-open or fail-closed.
3. On grant, the session (MAC, token, venue, expiry) is written to the local sqlite store, a `session_start` sync event is queued, and the agent redirects the browser to openNDS's auth endpoint: the `authaction` URL openNDS supplied, or the well-known `http://<gatewayaddress>:<gatewayport>/opennds_auth/` path. The redirect echoes `tok=<hid>` and `redir=<originurl>`, and openNDS then installs the firewall rule that lets the client's traffic through.
4. On deny, the agent responds `403` and does not call back to openNDS, so the client stays captive.

## Background tasks

Spawned by `Application::start`, independent of the request path:

- Heartbeat (`ru5ty-gate-heartbeat`): every `heartbeat.interval_secs`, POST uptime and active session count to the central API. Best effort; a failed post retries on the next tick.
- Sync (`ru5ty-gate-heartbeat`): every `sync.interval_secs`, pull up to `sync.batch_size` pending events from the local queue and push them to the central API. Events stay queued until acknowledged, so an outage does not lose session history.
- Expiry sweep (`jobs/expiry_sweep_job.rs`): every 60 seconds, delete expired sessions so the local store does not grow unbounded on a long-lived device.

## Run

```bash
cp config/agent.example.toml config/agent.toml
cargo run -p ru5ty-gate-agent -- --config config/agent.toml
```

From the repository root, `pnpm dev:agent` runs the same command against `apps/backend/agent/config/agent.toml`.

Config resolution order: `--config <path>`, then `$RU5TY_GATE_CONFIG`, then `config/agent.toml` relative to the working directory. `config/agent.toml` and the local `data/` directory are git-ignored because the config can hold the central API key.

`config/agent.router.example.toml` holds the settings for the router: bind address `0.0.0.0:2080` and the database under `/var/lib/ru5ty-gate/`.

Point openNDS's FAS settings (`fas_remotefasurl` or `fas_url` in `/etc/config/opennds`) at the agent's `/fas` endpoint, for example `http://127.0.0.1:2080/fas` when running on the same router.

Logging goes through `ru5ty-gate-logging`: `RUST_LOG` or `LOG_LEVEL` select the level, and `APP_ENV=production` switches to JSON output.

## Layout

```
src/
  main.rs               binary entry: CLI, logging, configuration, start
  lib.rs                module declarations and re-exports
  application.rs        Application: wires store, central client and FAS router, runs tasks
  agent_error.rs        AgentError
  agent_version.rs      AGENT_VERSION
  cli.rs                Cli (--config)
  shutdown.rs           SIGINT and SIGTERM handling
  jobs/                 expiry sweep job
tests/
  agent.rs              CLI and initialisation tests
```

The FAS server, sqlite store, central client, settings and scheduled tasks live in the shared crates under `common/`: `agent-config`, `session-store`, `central-client`, `fas-server` and `heartbeat`.

## Deviations from the other backend services

This service runs on a router rather than behind the API gateway, so it differs from the Postgres-backed services in three ways:

- Configuration is a TOML file (`ru5ty-gate-agent-config`) instead of environment variables read through `EnvReader`.
- Storage is a local sqlite file instead of Postgres, so there are no SQLx migrations. The schema lives in `ru5ty-gate-session-store`.
- There is no Dockerfile and no Coolify deployment. The binary is cross-compiled for OpenWrt and installed on the device.

## Test

```bash
cargo test -p ru5ty-gate-agent
cargo clippy -p ru5ty-gate-agent --all-targets -- -D warnings
```
