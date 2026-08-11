# Ru5ty Gate

<p align="left">
  <img
    src="https://res.cloudinary.com/dbqg2azyd/image/upload/v1786455709/0809d997-dc1a-41fb-a86e-f3b23e5f809d.png"
    alt="Ru5ty Gate"
    width="250"
  />
</p>

Router-side daemon that runs alongside [openNDS](https://opennds.readthedocs.io/)
on captive-portal hardware (targeted at a GL-iNet GL-MT6000 running OpenWrt).
openNDS handles the actual captive redirect and splash page; this agent sits
behind it as the **Forwarding Authentication Service (FAS)** -- openNDS calls
out to it to approve or deny a client, and it handles session persistence,
heartbeat reporting, and buffered sync to a central platform.

This is a Rust take on the same protocol surface as the Go agent (Gosling
Systems spec) -- different runtime, same job.

## v1 scope

- Single venue / single gateway.
- Full FAS auth round-trip with openNDS -- this is the part actually worth
  proving out first; everything else is scaffolding around it.
- Local, sqlite-backed session store that survives restarts and central-API
  outages, with buffered session events synced to the central platform once
  it's reachable again.
- Basic heartbeat (uptime + active session count) posted to the central API
  on an interval.

Explicitly **out of scope** for v1: zero-PII consent capture / splash page
content (that's openNDS's job), multi-venue config, and advertiser
walled-garden sync. Those are deferred to the full Gosling spec if this
proves out.

## Workspace layout

```
crates/
  agent-config/    Settings struct + TOML loader (venue, central API, session/
                    heartbeat/sync defaults)
  session-store/    rusqlite wrapper: active sessions + buffered sync-event
                    queue, survives restarts and central-API outages
  central-client/   reqwest wrapper: validate sessions, pull policy, post
                    heartbeats, sync buffered events against the central API
  fas-server/       axum HTTP server -- the FAS endpoint openNDS calls, plus
                    /health and /status
  heartbeat/        Tokio scheduled tasks: periodic heartbeat + buffered
                    sync-event flush
  agent/            binary crate (`ru5ty-gate-agent`) that wires everything
                    together
config/
  agent.example.toml  example config -- copy to agent.toml and fill in
```

## Protocol: how the FAS round-trip works

1. A client on the gateway's network tries to reach the internet. openNDS
   intercepts and redirects the client's browser to this agent's `GET /fas`
   endpoint, with a query string describing the client and gateway
   (`clientip`, `clientmac`, `gatewayname`, `gatewayaddress`, `gatewayport`,
   `originurl`, `hid`, and optionally `authaction`/`gatewaymac`/`clientif`
   depending on openNDS's `fas_secure_enabled` level).
2. The agent asks the central platform whether to grant this client
   (`central-client::validate_session`), with a short timeout. If the
   central platform is unreachable, it falls back to local policy
   (`session.allow_offline` in config controls fail-open vs. fail-closed).
3. On grant: the session (MAC, token, venue, expiry) is written to the local
   sqlite store, a `session_start` sync event is queued, and the agent
   redirects the client's browser to openNDS's own auth endpoint (either the
   `authaction` URL openNDS supplied, or the well-known
   `http://<gatewayaddress>:<gatewayport>/opennds_auth/` path), echoing back
   `tok=<hid>` and `redir=<originurl>`. openNDS then installs the firewall
   rule that actually lets the client's traffic through.
4. On deny: the agent responds `403` and does not call back to openNDS, so
   the client stays captured.

Background tasks (spawned by the `agent` binary, implemented in `heartbeat`):

- **Heartbeat**: every `heartbeat.interval_secs`, POST uptime + active
  session count to the central API. Best-effort -- a failed post just
  retries next tick.
- **Sync**: every `sync.interval_secs`, pull up to `sync.batch_size` pending
  events from the local queue and push them to the central API; events stay
  queued until acknowledged, so an outage doesn't lose session history.
- **Expiry sweep**: periodically deletes expired sessions from the local
  store so it doesn't grow unbounded.

## Running

```sh
cp config/agent.example.toml config/agent.toml
# edit config/agent.toml: venue id, central API URL, bind address, db path

cargo run -p ru5ty-gate-agent
# or: cargo run -p ru5ty-gate-agent -- --config path/to/agent.toml
```

Config resolution order: `--config <path>` flag, then `$RU5TY_GATE_CONFIG`,
then `config/agent.toml`.

Point openNDS's FAS settings (`fas_remotefasurl` / `fas_url` in
`/etc/config/opennds` or the openNDS UCI config) at this agent's `/fas`
endpoint, e.g. `http://127.0.0.1:2080/fas` if running on the same router.

## Development

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```
