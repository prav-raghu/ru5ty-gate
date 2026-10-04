# Agent

Router-side daemon that runs alongside [openNDS](https://opennds.readthedocs.io/) on captive-portal hardware, targeted at a GL-iNet GL-MT6000 running OpenWrt. Dev ports `4009` (FAS) and `4010` (admin); on the router `2080` and `2081`.

openNDS handles the captive redirect and the splash page. This agent sits behind it as the Forwarding Authentication Service (FAS): openNDS calls out to it to approve or deny a client, and it handles session persistence, heartbeat reporting and buffered sync to the central platform.

This is a Rust take on the same protocol surface as the Go agent (Gosling Systems spec): a different runtime doing the same job.

## v1 scope

- Single venue and single gateway.
- Full FAS auth round-trip with openNDS at `fas_secure_enabled` level 1.
- A local sqlite session store that survives restarts and central-API outages, with buffered session events synced to the central platform once it is reachable again.
- A basic heartbeat (uptime, active sessions, queue depth, last successful sync) posted to the central API on an interval.

Out of scope for v1: splash page content and consent capture (openNDS ThemeSpec or the customer web app), multi-venue config, advertiser walled-garden sync, and FAS levels 0, 2, 3 and 4.

## Protocol: how the FAS round-trip works

This follows the openNDS `fas_secure_enabled '1'` flow.

1. A client on the gateway's network tries to reach the internet. openNDS intercepts it and redirects the browser to `GET /fas?fas=<base64>` on the agent.
2. The decoded `fas` value is a list of `name=value` pairs separated by `", "`: `clientip`, `clientmac`, `gatewayname`, `hid`, `gatewayaddress` (host:port), `authdir`, `originurl` (URL-encoded), `clientif` and others the agent ignores. The agent validates every field (see Security model) and rejects anything else with `400`.
3. The agent asks the central platform whether to grant this client (`ru5ty-gate-central-client::validate_session`) with a short timeout. If the central platform is unreachable it falls back to local policy; `session.allow_offline` selects fail-open or fail-closed.
4. On grant, the session is written to the local sqlite store, a `session_start` sync event is queued, and the agent redirects the browser to `http://<gatewayaddress>/<authdir>/?tok=<token>&redir=<url>`. The token is `sha256_hex(hid + faskey)`, which only an agent holding the `faskey` can produce, and openNDS then installs the firewall rule that lets the client through. The `redir` is the central redirect, else the cached policy redirect, else the client's `originurl`.
5. On deny, the agent responds `403` and does not call back to openNDS, so the client stays captive.

The session duration is the central `session_seconds`, else the cached policy duration, else `session.default_duration_secs`.

## Endpoints

| Listener | Method | Path | Purpose |
|---|---|---|---|
| public (`server.bind_addr`) | GET | `/fas` | openNDS auth callout, rate limited per client IP |
| public | GET | `/health` | liveness probe |
| admin (`server.admin_bind_addr`, loopback) | GET | `/status` | active sessions, queue depth, `clock_trusted` |
| admin | POST | `/binauth` | openNDS BinAuth hook reports a deauthentication (`{"mac","method"}`) |

Captive clients can reach only the public listener, so `/status` and `/binauth` are not exposed to them.

## Security model

- The `faskey` is the shared secret with openNDS. The agent refuses to start with a key shorter than 16 characters, and `REPLACE_ME` placeholders in the example configs fail that check on purpose.
- `clientip` in the payload must equal the TCP peer address (`fas.verify_client_ip`), so one device cannot create or overwrite another device's session.
- Fields are validated: MAC format, IP literal, `hid` 1 to 128 alphanumerics, `gatewayaddress` as `ip:port`, `authdir` as `[A-Za-z0-9_-]{1,64}`, `originurl` as an http(s) URL of at most 2,048 characters. The redirect is built only from validated values, so there is no open redirect.
- Requests have a timeout, a concurrency limit and a per-client rate limit. Responses carry `Cache-Control: no-store`.
- `central.api_key`, `fas.faskey` and `privacy.hash_pepper` are held as secrets and never appear in `Debug` output. An `api_key` over plain `http` is rejected unless the host is loopback or `central.allow_insecure_http` is set.
- Client MAC and IP addresses are personal information. By default (`privacy.send_raw_identifiers = false`) the central platform receives salted hashes, and logs show only a short hash or a masked MAC.
- Configuration is validated at start: zero intervals, bad addresses, short secrets and an oversized batch all stop the agent before it binds a port.

## Background tasks

Spawned by `Application::start` and supervised: if one stops, the agent exits with an error so procd restarts it.

- Heartbeat (`ru5ty-gate-heartbeat`): every `heartbeat.interval_secs`, POST uptime, active sessions, queue depth and last successful sync to the central API. Best effort.
- Policy (`ru5ty-gate-heartbeat`): on the same interval, fetch the venue policy and cache it in sqlite, so it still applies while the central platform is down.
- Sync (`ru5ty-gate-heartbeat`): every `sync.interval_secs`, push up to `sync.batch_size` pending events. Events stay queued until acknowledged. Each tick also drops the oldest events beyond `sync.max_pending` and deletes synced events older than `sync.retain_synced_secs`.
- Expiry sweep (`jobs/expiry_sweep_job.rs`): every 60 seconds, remove expired sessions, queue a `session_end` event for each, and run `ndsctl deauth <mac>` when `enforcement.ndsctl_path` is set. The sweep does nothing while the clock is untrusted (see below).

## Session end and enforcement

The agent tells openNDS to end a session only at expiry (`ndsctl deauth`). It cannot extend a session beyond openNDS's own `sessiontimeout`, so set that to at least the longest session you grant. The reverse direction, openNDS ending a session first (timeout, quota, `ndsctl deauth`), reaches the agent through the BinAuth hook in `openwrt/custombinauth.sh`, which posts to `/binauth` and produces a `session_end` event.

## Clock

Routers without a battery-backed clock boot with a wrong time. The agent treats the clock as untrusted while it is earlier than the binary's build time. Sessions granted in that window are flagged, are skipped by the sweep, and get their full duration restarted from the first sweep after the clock becomes valid. `/status` reports `clock_trusted`.

## Run

```bash
cp config/agent.example.toml config/agent.toml
# replace every REPLACE_ME (openssl rand -hex 32)
cargo run -p ru5ty-gate-agent -- --config config/agent.toml
```

From the repository root, `pnpm dev:agent` runs the same command against `apps/backend/agent/config/agent.toml`.

Config resolution order: `--config <path>`, then `$RU5TY_GATE_CONFIG`, then `config/agent.toml` relative to the working directory. `config/agent.toml` and the local `data/` directory are git-ignored because the config holds secrets.

`ru5ty-gate-agent healthcheck` probes `/health` on the configured listener and exits 0 or 1, for procd or a container healthcheck.

Logging goes through `ru5ty-gate-logging`: `RUST_LOG` or `LOG_LEVEL` select the level, and `APP_ENV=production` switches to JSON output.

## Deploy to a router

See [openwrt/README.md](openwrt/README.md) for the cross-build, the procd service, the BinAuth hook and the openNDS settings. `config/agent.router.example.toml` holds the router settings.

## Layout

```
src/
  main.rs               binary entry: CLI, logging, configuration, start
  lib.rs                module declarations and re-exports
  application.rs        Application: wires store, central client and both routers, supervises tasks
  agent_error.rs        AgentError
  agent_version.rs      AGENT_VERSION
  cli.rs, command.rs    Cli (--config) and the healthcheck subcommand
  healthcheck.rs        run_healthcheck
  shutdown.rs           SIGINT and SIGTERM handling
  jobs/                 expiry sweep job
openwrt/                init script, BinAuth hook, openNDS UCI example, deployment guide
tests/
  unit/                 CLI, initialisation, sweep, healthcheck and BinAuth hook tests
  integration/          the real Application against a stub central server and openNDS-shaped requests
  common/               shared helpers
```

The FAS server, sqlite store, central client, settings, scheduled tasks and `ndsctl` runner live in the shared crates under `common/`: `agent-config`, `session-store`, `central-client`, `fas-server`, `heartbeat` and `ndsctl`.

## Deviations from the other backend services

This service runs on a router rather than behind the API gateway, so it differs from the Postgres-backed services in three ways:

- Configuration is a TOML file (`ru5ty-gate-agent-config`) instead of environment variables read through `EnvReader`.
- Storage is a local sqlite file instead of Postgres, so there are no SQLx migrations. The schema lives in `ru5ty-gate-session-store`.
- There is no Dockerfile and no Coolify deployment. The binary is cross-compiled for OpenWrt.

## Test

```bash
cargo test -p ru5ty-gate-agent
cargo clippy -p ru5ty-gate-agent --all-targets -- -D warnings
```
