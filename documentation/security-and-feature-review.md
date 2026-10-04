# Security and Feature Review

Review of the ru5ty-gate monorepo at commit `18d04ee` (branch `ccr-8ab2d8b4-ea6b21`), carried out on 2026-10-04. It is written as a work queue for Claude Sonnet: every item is self-contained, with the location, the evidence, the fix and the acceptance criteria.

## How to work through this document

1. Read `CLAUDE.md` first. Every rule there still applies: no code comments, one item per file, tests in `tests/`, strict clippy, conventional commit messages (the husky `commit-msg` hook enforces them).
2. Take one item per commit, in the order given in [Suggested order](#suggested-order). Prefix each commit subject with the item id, for example `fix(agent): SEC-03 stop open redirect through authaction`.
3. Before marking an item done, run the gates that apply to it:
   - Rust: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test -p <crate>`
   - TypeScript: `pnpm typecheck`, `pnpm lint`, `pnpm format:check`, `pnpm test`
   - Supply chain: `cargo deny check`, `pnpm audit --audit-level=high`
4. Tick the item in [Status](#status) when it is merged.
5. Model choice: per `CLAUDE.md`, never delegate security, auth or cross-cutting `common/*` work to Haiku. Every `SEC-*` item and every `FEAT-*` item marked "security-relevant" stays on Sonnet or the session model. The suggested subagent for each item is listed with it.

Severity scale: Critical (exploitable now, or the product does not work), High, Medium, Low.

## Status

| Id | Title | Severity | Area | Done |
|---|---|---|---|---|
| SEC-01 | Agent does not speak any secure openNDS FAS level | Critical | agent | [x] |
| SEC-02 | Any LAN client can forge or overwrite session records | High | agent | [x] |
| SEC-03 | Open redirect through `authaction` | Medium | agent | [x] |
| SEC-04 | Unbounded input and unbounded sqlite growth | Medium | agent | [x] |
| SEC-05 | Zero or invalid config values panic tasks or force fail-open | Medium | agent | [x] |
| SEC-06 | Central API key can travel over plain HTTP and appears in `Debug` output | Medium | agent | [x] |
| SEC-07 | `/status` is reachable by captive clients | Low | agent | [x] |
| SEC-08 | MAC and IP addresses logged and synced without minimisation (POPIA) | Medium | agent | [x] |
| SEC-09 | Agent router has no timeout, concurrency or rate limit layers | Medium | agent | [x] |
| SEC-10 | Next.js and other JS dependencies with critical and high advisories | Critical | frontend | [x] |
| SEC-11 | `ClientIp` trusts the left-most `X-Forwarded-For` | High | platform | [x] |
| SEC-12 | Redis TLS certificate verification is off by default | Medium | platform | [x] |
| SEC-13 | Gateway `/metrics` is publicly routed without auth | Medium | platform | [x] |
| SEC-14 | GitHub Actions token permissions and action pinning | Medium | CI | [x] |
| SEC-15 | Security scan workflow is fully commented out | Medium | CI | [x] |
| FEAT-01 | Central platform endpoints for the agent | Critical | platform | [x] |
| FEAT-02 | Session end events and openNDS deauth ingestion | High | agent | [x] |
| FEAT-03 | Session duration is never enforced on the network | High | agent | [x] |
| FEAT-04 | Central policy and landing redirect are ignored | Medium | agent | [x] |
| FEAT-05 | Config validation for the agent | Medium | agent | [x] |
| FEAT-06 | OpenWrt build, packaging and service script | High | agent / CI | [x] |
| FEAT-07 | Router clock sanity before granting sessions | Medium | agent | [x] |
| FEAT-08 | Agent observability: queue depth, healthcheck subcommand | Low | agent | [x] |
| FEAT-09 | Admin dashboard for venues, gateways and sessions | Medium | frontend | [x] |
| FEAT-10 | Splash and consent page | Medium | frontend | [ ] (deferred) |
| FEAT-11 | End-to-end test against real openNDS request shapes | High | agent | [x] |

Implementation notes (2026-10-04):

- Every item is implemented and committed except FEAT-10, which waits on the decisions listed under [Decisions needed from Prav](#decisions-needed-from-prav).
- Not verified: the GitHub Actions workflows (SHA pins, the `agent-openwrt` cross-build job and the re-enabled `security-scan.yml`) have not run on GitHub, and the agent has not been run on a real GL-MT6000 with openNDS. The OpenWrt packaging files are untested on hardware.
- SEC-10: two high advisories have no published fix and are ignored through `auditConfig.ignoreGhsas` in `pnpm-workspace.yaml`. They are `node-forge` (through Expo tooling) and `braces` (through changesets). Both are build-time tooling and neither ships in a runtime bundle. Remove the entries once fixed versions exist.
- SEC-01: only openNDS FAS level 1 is supported. Level 0 was dropped, so there is no `allow_insecure_level0` setting. Level 2 (AES) is not implemented.
- FEAT-09: the admin-web access token lives in memory only (project rule). `admin-api` sets the refresh token as an `HttpOnly`, `SameSite=Strict` cookie (`Secure` in production), so a page reload restores the session through `/auth/refresh`, a 401 triggers one silent refresh and retry, and sign out calls `/auth/logout`, which clears the cookie. The refresh token is still returned in the JSON body for non-browser clients.
- FEAT-09: the UI hides write actions using the `permissions` list that `/auth/me` now returns. The backend `venue:read` and `venue:write` checks remain the authority.
- FEAT-09: the template demo pages and components (`Home`, `About`, `CounterCard`, `ApiTestCard`, `TailwindShowcase` and their stores) were removed. The primary and destructive colour tokens were darkened to meet WCAG AA contrast.

## Method

- Read all agent code (`apps/backend/agent`, `common/{agent-config,session-store,central-client,heartbeat,fas-server}`) line by line.
- Ran the agent binary and reproduced every agent finding with `curl` (commands below).
- Checked the agent against the upstream openNDS FAS documentation (`docs/source/fas.rst`) and the reference scripts `forward_authentication_service/fas-hid/fas-hid.php` and `fas-aes/fas-aes.php` on openNDS `master`.
- Targeted review of the template-derived platform: SQL construction, CORS, JWT secrets, client IP handling, Redis TLS, gateway routes, frontend token storage and XSS sinks, Docker Compose, nginx, GitHub workflows.
- Tools: `cargo deny check` (passes), `pnpm audit` (3 critical, 64 high, 34 moderate, 1 low), secret pattern scan over tracked files (no real secrets; only placeholders in `*.example` files).

Things that were checked and are fine:

- All repository SQL binds values; identifiers come from compile-time constants (`common/database/src/repositories/repository.rs`). No dynamic SQL strings were found.
- JWT secrets have a minimum length check (`common/auth/src/auth_config.rs`). Auth signs HS256 only.
- CORS uses an explicit origin list with credentials, never a wildcard (`common/http/src/cors.rs`).
- GraphQL has depth and complexity limits, and introspection is off in production.
- No `dangerouslySetInnerHTML`, `eval` or tokens in `localStorage` in the frontends (only the theme preference).
- No committed `.env` files and no real keys in tracked files.

---

## Agent security findings

### SEC-01 Agent does not speak any secure openNDS FAS level

- Severity: Critical
- Where: `common/fas-server/src/fas_query.rs`, `common/fas-server/src/controllers/fas_controller.rs`, `common/agent-config/src/*`
- Subagent: `backend-service` (security-relevant, keep on Sonnet)

Problem: `FasQuery` expects flat query parameters (`clientip`, `clientmac`, `gatewayname`, `gatewayaddress`, `gatewayport`, `originurl`, `hid`) and returns `tok=<hid>`. Current openNDS never sends that shape:

| openNDS level | What openNDS sends | What the FAS must return |
|---|---|---|
| 0 | `authaction=http://gw:port/opennds_auth/?clientip=..&gatewayname=..&tok=<token>&redir=..` (token in clear text) | follow `authaction` |
| 1 (default), 4 | `fas=<base64>` where the decoded text is `name=value` pairs separated by `", "`: `clientip`, `clientmac`, `client_type`, `gatewayname`, `gatewayurl`, `version`, `hid`, `gatewayaddress` (host:port), `gatewaymac`, `authdir`, `originurl`, `clientif`, `cpi_query`, plus custom params | `http://{gatewayaddress}/{authdir}/?tok={sha256_hex(hid + faskey)}&redir={url}&custom={base64}` |
| 2, 3 | `fas=<data>&iv=<iv>`, AES-256-CBC with the pre-shared `faskey` | same `tok` as level 1 (level 3 uses authmon instead of a redirect) |

openNDS's own documentation says level 0 "is easy to bypass ... Generally, it should not be used". So the agent either fails every real request, or only works with an insecure, bypassable configuration.

Evidence (agent running on port 4019):

```bash
curl -s "http://127.0.0.1:4019/fas?fas=$(printf 'hid=abc, clientip=1.2.3.4, clientmac=aa:bb:cc:dd:ee:ff, gatewayname=gw, gatewayaddress=10.0.0.1:2050, authdir=opennds_auth, originurl=http%%3A%%2F%%2Fx' | base64 -w0)"
# Failed to deserialize query string: missing field `clientip` -> 400
```

Fix:

1. Add a `faskey` setting to `ru5ty-gate-agent-config` (`[fas] secure_level = 1`, `faskey = "..."`). Treat `faskey` as a secret (see SEC-06). Refuse to start when `secure_level >= 1` and `faskey` is empty.
2. Replace `FasQuery` with a raw `FasRequest { fas: String, iv: Option<String> }` extractor plus a `FasPayload` struct (one file each) that holds the decoded fields. Add a parser (`fas_payload_parser.rs`) that base64-decodes, splits on `", "`, splits each pair on the first `=`, and URL-decodes `originurl` and `gatewayurl`. Unknown names go into a `custom: BTreeMap<String, String>`.
3. Build the return URL as `http://{gatewayaddress}/{authdir}/?tok={sha256_hex(hid + faskey)}&redir={redir}`. Use the `sha2` and `hex` crates, which are already in the workspace through `ru5ty-gate-logging`. The hash input is the plain string concatenation `hid + faskey`, and the output is lowercase hex, matching PHP's `hash('sha256', $hid.$key)`.
4. Support level 1 first. Treat level 2 as a follow-up item. It needs AES-256-CBC (`aes` + `cbc` crates) with byte-for-byte compatibility with PHP's `openssl_decrypt(base64_decode($fas), "AES-256-CBC", $key, 0, $iv)`. Note that option `0` makes PHP base64-decode the input a second time, and PHP pads or truncates the key to 32 bytes. Build the test vectors with PHP or the openNDS C source; do not guess.
5. Reject level 0 unless `[fas] allow_insecure_level0 = true` is set, and log a warning at startup when it is.
6. Update `common/fas-server/README.md`, `apps/backend/agent/README.md` and both example configs. The openNDS UCI settings to document are `fas_secure_enabled '1'` and a non-default `faskey`.

Acceptance:

- `tests/fas_payload_parser.rs`: decodes a payload built exactly like the openNDS example, including URL-encoded `originurl` and a custom parameter; returns an error for malformed base64, missing `hid`, missing `clientmac` or missing `gatewayaddress`.
- `tests/fas_server.rs`: a level 1 request produces a 302 to `http://10.0.0.1:2050/opennds_auth/?tok=<expected sha256>&redir=...`, with the hash checked against a fixed vector (for example `sha256("abc" + "secret")`).
- Level 0 style flat parameters return 400 unless `allow_insecure_level0` is set.

### SEC-02 Any LAN client can forge or overwrite session records

- Severity: High
- Where: `common/fas-server/src/controllers/fas_controller.rs`, `common/session-store/src/session_store.rs`
- Subagent: `backend-service` (security-relevant)

Problem: `/fas` is unauthenticated, and the agent trusts every field in the query string. Any device on the captive network can:

- create a session record and a `session_start` sync event for any MAC address, which pollutes analytics and billing on the central platform;
- overwrite another client's row, because `upsert_session` replaces by MAC (token, venue and expiry are all replaced);
- while the central platform is unreachable and `allow_offline = true`, get every forged request "granted" locally.

openNDS itself still decides who gets network access. The damage is to the integrity of the records the central platform receives, and to the router's storage.

Evidence:

```bash
curl -s -o /dev/null -w '%{http_code}\n' "http://127.0.0.1:4019/fas?clientip=9.9.9.9&clientmac=AA:BB:CC:DD:EE:01&gatewayname=gw&gatewayaddress=10.0.0.1&gatewayport=2050&originurl=http://x&hid=attacker"
# 302: the existing session for AA:BB:CC:DD:EE:01 now holds token "attacker"
```

Fix (after SEC-01):

1. Serve the router with `into_make_service_with_connect_info::<SocketAddr>()` in `apps/backend/agent/src/application.rs`. In the controller, reject the request (403) when the TCP peer IP is not the decoded `clientip`. The client's browser talks to the agent directly on the LAN, so the two must match. Make this check switchable (`[fas] verify_client_ip = true`) for setups with a reverse proxy.
2. Only write the session and enqueue `session_start` after the payload is validated (SEC-01 and SEC-04) and the central decision is made.
3. Optional, stronger: confirm the MAC and hid with `ndsctl json <mac>` before granting. Put this behind a trait so tests can replace it.

Acceptance: a test in `tests/fas_server.rs` that builds the router with a mismatching `ConnectInfo` peer gets 403 and leaves the store unchanged. A matching peer still gets 302.

### SEC-03 Open redirect through `authaction`

- Severity: Medium
- Where: `common/fas-server/src/fas_query.rs` (`auth_redirect_url`)
- Subagent: `backend-service`

Problem: when `authaction` is present, it becomes the base of the `Location` header unchecked. A link to the trusted portal host can bounce a user to any site, together with their `tok`.

Evidence:

```bash
curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' "http://127.0.0.1:4019/fas?clientip=1.2.3.4&clientmac=AA:BB:CC:DD:EE:01&gatewayname=gw&gatewayaddress=10.0.0.1&gatewayport=2050&originurl=http://x&hid=h&authaction=https://evil.example/phish"
# 302 https://evil.example/phish?tok=h&redir=http%3A%2F%2Fx
```

Fix: build the auth URL only from `gatewayaddress` and `authdir`, both from the verified payload (SEC-01). Validate `authdir` against `^[A-Za-z0-9_-]{1,64}$`, and parse `gatewayaddress` as `host:port` where the host is an IP literal. If level 0 support is kept, accept `authaction` only when its scheme is `http`, its host and port equal `gatewayaddress`, and its path is `/{authdir}/`.

Acceptance: tests showing that an `authaction` on another host is rejected (400), and that a URL-encoded `gatewayaddress` such as `evil.example%2F` is rejected.

### SEC-04 Unbounded input and unbounded sqlite growth

- Severity: Medium
- Where: `common/fas-server`, `common/session-store/src/session_store.rs`, `common/heartbeat/src/sync_task.rs`
- Subagent: `backend-service`, then `common-packages` for the store

Problem:

- No field has a length or format check. A 20,000 character MAC was accepted, stored and queued for sync (reproduced).
- `mark_synced` only sets `synced_at`. Synced rows are never deleted, so `sync_queue` grows forever on router flash.
- While the central platform is down, every request adds a pending row with no cap.

Fix:

1. Add `validator` derives to the payload struct (CLAUDE.md requires `validator`, not hand-written checks): `clientmac` matches `^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$`, `clientip` parses as `IpAddr`, `hid` is 1 to 128 hex characters, URLs are at most 2,048 characters, `gatewayname` at most 64 characters. Do not use `deny_unknown_fields` on the outer request, because openNDS appends custom parameters.
2. Add `SessionStore::prune_synced(older_than: i64) -> Result<usize>` and call it from the expiry sweep job. Keep synced rows for a configurable retention (`[sync] retain_synced_secs`, default 86,400).
3. Add `[sync] max_pending` (default 50,000). When the queue is full, drop the oldest pending rows and log at `warn` with the number dropped.
4. Normalise MACs to lower case before storing, so `AA:..` and `aa:..` are the same client.

Acceptance: store tests for pruning and for the cap; a fas-server test where an over-long MAC gets 400 and nothing is written.

### SEC-05 Zero or invalid config values panic tasks or force fail-open

- Severity: Medium
- Where: `common/agent-config`, `common/heartbeat`, `apps/backend/agent/src/application.rs`
- Subagent: `backend-service`

Problem: `tokio::time::interval` panics on a zero period. With `heartbeat.interval_secs = 0`, the heartbeat task panics (reproduced: `panicked at common/heartbeat/src/heartbeat_task.rs:16:26`). The process keeps running, but heartbeats stop silently. `sync.interval_secs = 0` does the same to sync. `central.timeout_secs = 0` makes every central call fail, so with `allow_offline = true` every client is granted from local policy. `sync.batch_size = 0` means nothing ever syncs.

Fix: see FEAT-05. Validation must happen in `Settings::parse`, before anything starts. Also make the agent exit non-zero when a background task's `JoinHandle` finishes with a panic. Use `tokio::select!` over the server future and the three handles in `Application::start`, so a dead task can never go unnoticed.

Acceptance: `Settings::parse` returns a `ConfigError::Invalid { field, reason }` for each zero value, and a test proves it. An agent test proves `start` returns an error when a task panics.

### SEC-06 Central API key can travel over plain HTTP and appears in `Debug` output

- Severity: Medium
- Where: `common/agent-config/src/central_settings.rs`, `common/central-client/src/client_config.rs`
- Subagent: `backend-service`

Problem:

- `base_url` defaults to `http://127.0.0.1:8080`, and nothing stops an `http://` URL in production, so the bearer token can cross the internet in clear text.
- `CentralSettings`, `Settings`, `ClientConfig` and `CentralClient` all derive `Debug` with `api_key` inside, so one `?settings` in a log line leaks the key.
- The config file holds the key, but nothing documents or enforces file permissions on the router.

Fix:

1. Wrap `api_key` (and `faskey` from SEC-01) in `secrecy::SecretString`, and expose them only at the point of use (`bearer_auth(key.expose_secret())`).
2. In validation (FEAT-05), reject a non-`https` `base_url` when `api_key` is set, unless `[central] allow_insecure_http = true`. Allow `http://127.0.0.1` and `http://localhost` for development.
3. Document `chmod 600 /etc/ru5ty-gate/agent.toml` and owner `root` in the agent README and the OpenWrt package (FEAT-06).

Acceptance: a test that `format!("{settings:?}")` does not contain the key, and a validation test for the HTTPS rule.

### SEC-07 `/status` is reachable by captive clients

- Severity: Low
- Where: `common/fas-server/src/router.rs`
- Subagent: `backend-service`

Problem: the FAS server binds `0.0.0.0:2080` so that clients can reach `/fas`. That also exposes `/status` (venue id, active session count, queue depth) to every captive client.

Fix: move `/status` (and future diagnostics) to a second listener bound to `127.0.0.1` (`[server] admin_bind_addr`, default `127.0.0.1:2081`), or require the peer IP to be loopback. Keep `/health` on the public listener.

Acceptance: a test that a non-loopback `ConnectInfo` gets 404 or 403 on `/status`.

### SEC-08 MAC and IP addresses logged and synced without minimisation (POPIA)

- Severity: Medium (compliance)
- Where: `common/fas-server/src/controllers/fas_controller.rs` (log lines and the `session_start` payload), `common/heartbeat`
- Subagent: `backend-service`. The policy decision belongs to Prav.

Problem: client MAC and IP addresses are personal information under POPIA. They are logged at `info` on every grant or deny, stored in sqlite and sent to the central platform, with no retention limit and no hashing. The README states zero-PII consent capture is out of scope for v1, but the agent still collects identifiers.

Fix:

1. In logs, replace raw MACs with a keyed hash. `ru5ty-gate-logging` already has `hash_ip`; add an equivalent `hash_mac` there. Keep raw values out of `info` level entirely.
2. Make the payload sent to the central platform configurable (`[privacy] send_raw_identifiers`, default `false`, which sends hashed MAC and IP).
3. Document the lawful basis and the retention period in `documentation/` once Prav decides them.

Acceptance: a test that grant and deny log lines do not contain the raw MAC; with `send_raw_identifiers = false`, the sync payload contains no raw MAC or IP.

### SEC-09 Agent router has no timeout, concurrency or rate limit layers

- Severity: Medium
- Where: `common/fas-server/src/router.rs`, `apps/backend/agent/src/application.rs`
- Subagent: `backend-service`

Problem: the platform services use the middleware stack described in `.claude/rules/backend.md` (catch panic, security headers, rate limit, request logger). The agent uses none of it. A single client can open many slow connections, or hammer `/fas`, on a small router.

Fix: wrap the router in `tower::ServiceBuilder` with `TimeoutLayer` (10 s), `ConcurrencyLimitLayer` (64), `catch_panic_layer()` and `security_headers` from `ru5ty-gate-http`, and a per-client-IP rate limit on `/fas` (for example 10 per minute). Reuse `ru5ty-gate-http`'s limiter only if it does not pull Redis into the router binary; otherwise use a small in-memory limiter keyed on the peer IP. Never key it on `X-Forwarded-For` (see SEC-11).

Acceptance: a test that the eleventh `/fas` request from one peer within a minute gets 429.

---

## Platform security findings

### SEC-10 Next.js and other JS dependencies with critical and high advisories

- Severity: Critical
- Where: `apps/frontend/customer-web`, `apps/frontend/admin-web`, `apps/mobile/customer-mobile`, root dev tooling. Lockfile `pnpm-lock.yaml`.
- Subagent: `frontend-nextjs` and `frontend-react` (security-relevant)

`pnpm audit` reports 3 critical, 64 high and 34 moderate advisories. The ones that matter at runtime:

| Package (via) | Fix version | Advisories |
|---|---|---|
| `next` (customer-web) | at least 16.3.6 | 3 critical RCE (Image Optimization with AVIF, `next/og` ImageResponse, Windows hosts), middleware bypass, SSRF in Server Actions and rewrites, DoS |
| `sharp`, `postcss` (customer-web through next) | sharp at least 0.35.0, postcss at least 8.5.23 | libvips CVEs, arbitrary file read |
| `react-router` (admin-web) | at least 7.18.2 | open redirect, XSS, CSRF bypass, DoS |
| `axios`, `form-data` (admin-web) | axios at least 1.18.0, form-data at least 4.0.6 | prototype pollution, CRLF injection |
| `@xmldom/xmldom`, `node-forge` (mobile, through Expo tooling) | latest | build-time tooling |
| `brace-expansion`, `js-yaml`, `fast-uri`, `nanoid` | latest | dev-tooling DoS |

Fix: `pnpm --filter customer-web up next@latest @sentry/nextjs@latest`, `pnpm --filter admin-web up react-router-dom@latest axios@latest`, then `pnpm up -r` for the rest. Where a transitive version cannot move, add `pnpm.overrides` in the root `package.json`. Re-run all frontend gates and tests.

Acceptance: `pnpm audit --audit-level=high` exits 0, or any remaining advisory is listed here with the reason it cannot be fixed yet.

### SEC-11 `ClientIp` trusts the left-most `X-Forwarded-For`

- Severity: High
- Where: `common/http/src/extractors/client_ip.rs`, used by `common/http/src/middleware/rate_limit.rs`, request logging and audit logging. `apps/backend/api-gateway/src/controllers/proxy_controller.rs` appends to an existing header.
- Subagent: `common-packages` (security-relevant, keep on Sonnet)

Problem: `ClientIp::resolve` returns the first entry of `X-Forwarded-For`. The gateway appends the real peer to whatever the client sent, so the first entry is attacker-controlled. Anyone can send a different `X-Forwarded-For` on every request to bypass the rate limit and the login lockout, and to put a fake IP in audit logs.

Fix: add `TRUSTED_PROXY_HOPS` (default `1`, the gateway) read through `EnvReader` in each service's `ServiceConfig`. Resolve the client as the entry at position `len - hops` from the right of the chain, falling back to `ConnectInfo`. At the gateway (hops `0` for its own view), ignore any incoming `X-Forwarded-For` from the internet unless the peer is the configured edge proxy (Traefik), and write a fresh header.

Acceptance: unit tests in `common/http/tests/` for chains of 0, 1 and 3 entries, including a spoofed left-most entry. An admin-api integration test where rotating `X-Forwarded-For` values still hits 429.

### SEC-12 Redis TLS certificate verification is off by default

- Severity: Medium
- Where: `apps/backend/{admin,customer,schedule}-api/src/config/service_config.rs` (`redis_tls_reject_unauthorized` is `false` unless set to `"true"`), `docker-compose.yaml` (`REDIS_TLS_REJECT_UNAUTHORIZED: ${REDIS_TLS_REJECT_UNAUTHORIZED:-false}`), `common/cache/src/redis_url.rs` (adds `#insecure`)
- Subagent: `common-packages`

Problem: with a `rediss://` URL, certificate validation is disabled unless someone opts in. That allows a man-in-the-middle on the Redis connection, which carries the token denylist and the lockout state.

Fix: invert the default. Verify unless `REDIS_TLS_REJECT_UNAUTHORIZED=false` is set explicitly, change the compose default to `true`, and log a startup `warn` when verification is disabled.

Acceptance: config tests for unset, `true` and `false`.

### SEC-13 Gateway `/metrics` is publicly routed without auth

- Severity: Medium
- Where: `apps/backend/api-gateway/src/application.rs` (metrics are always installed and merged into the public router), `apps/backend/api-gateway/src/routes/metrics_route.rs`
- Subagent: `backend-service`

Problem: Prometheus metrics (route names, status counts, latencies) are served on the internet-facing gateway.

Fix: serve `/metrics` on a separate internal listener (`METRICS_PORT`, not published in `docker-compose.yaml`), or require a bearer token (`METRICS_TOKEN`, compared with `subtle::ConstantTimeEq` as schedule-api already does for its API key).

Acceptance: an integration test that `/metrics` on the public router returns 404 or 401.

### SEC-14 GitHub Actions token permissions and action pinning

- Severity: Medium
- Where: `.github/workflows/continuous-integration.yml`, `version-control.yml` (no top-level `permissions:`), all workflows (actions pinned to mutable tags; `SonarSource/sonarcloud-github-action@master` and `sonarsource/sonarqube-quality-gate-action@master`)
- Subagent: `infrastructure`

Fix: add top-level `permissions: contents: read` to every workflow and grant more per job only where needed (the docker job already does). Pin every third-party action to a full commit SHA with the tag in a trailing YAML comment, and replace the `@master` references. Enable Dependabot for `github-actions` in `.github/dependabot.yml` so the pins are kept current.

Acceptance: `grep -n '@master\|@v[0-9]*$' .github/workflows/*.yml` returns nothing, and every workflow has a top-level `permissions` block.

### SEC-15 Security scan workflow is fully commented out

- Severity: Medium
- Where: `.github/workflows/security-scan.yml`
- Subagent: `infrastructure`

Problem: the `pnpm audit`, CodeQL and TruffleHog jobs exist but every line is commented out, so none of them run.

Fix: re-enable the workflow after SEC-10, so `pnpm audit --audit-level=high` passes from day one. Keep CodeQL for `javascript-typescript`, and add a `cargo deny check advisories` job on a weekly schedule, because new RustSec advisories appear without code changes.

Acceptance: the workflow runs green on a pull request.

---

## Missing features

### FEAT-01 Central platform endpoints for the agent

- Priority: Critical (the agent has nothing to talk to)
- Where: `apps/backend/admin-api` (or a new `venue-api` if Prav prefers), `common/database/migrations`, `common/database/src/repositories`
- Subagents: `domain-modeler`, then `api-builder`, `jwt-security` for device auth (security-relevant)

The agent calls four endpoints, documented in `common/central-client/README.md`. None of them exist:

| Method | Path | Body | Response |
|---|---|---|---|
| POST | `/v1/venues/{venue_id}/sessions/validate` | `ValidateSessionRequest` | `ValidateSessionResponse { allow, session_seconds?, redirect_url?, reason? }` |
| GET | `/v1/venues/{venue_id}/policy` | none | `PolicyResponse { session_duration_secs, redirect_url? }` |
| POST | `/v1/venues/{venue_id}/heartbeat` | `HeartbeatRequest` | 2xx |
| POST | `/v1/venues/{venue_id}/sync` | `SyncBatchRequest` | `SyncBatchResponse { accepted }` |

Build:

1. Tables (each with the six base metadata columns from `rules/database.md`): `venues`, `gateways` (venue, name, hashed API key, last heartbeat), `captive_sessions`, `gateway_heartbeats`, `session_events` (with a unique key on gateway plus the agent's queue id, so a retried `/sync` is idempotent).
2. Device authentication: one API key per gateway, stored as a hash (argon2 through `ru5ty-gate-utilities`), checked in a guard against `venue_id`, compared in constant time. Never reuse user JWTs for devices.
3. The wire format here is snake_case. The rest of the platform uses camelCase. Either keep snake_case for this device API, documented as an exception, or change the agent DTOs in the same change. Do not leave them mismatched.
4. Service-layer tests in `tests/services` with `#[sqlx::test]`, as `CLAUDE.md` requires.

Acceptance: an integration test runs the real `CentralClient` against the service router and covers validate, policy, heartbeat and a retried sync without duplicates.

### FEAT-02 Session end events and openNDS deauth ingestion

- Priority: High
- Where: `common/session-store`, `apps/backend/agent/src/jobs/expiry_sweep_job.rs`, new agent route
- Subagent: `backend-service`

`SyncEventKind::SessionEnd` exists but nothing ever enqueues it. The expiry sweep deletes sessions silently, and when openNDS deauthenticates a client (timeout, quota, `ndsctl deauth`) the agent is never told.

Build: have the sweep enqueue `session_end` (reason `expired`) for each row it deletes, in the same transaction. Add a loopback-only `POST /binauth` route (on the SEC-07 admin listener), and ship a `custombinauth.sh` snippet that calls it with the MAC, token and reason openNDS passes to BinAuth. openNDS v10.1+ always runs BinAuth.

Acceptance: store and sweep tests, and a route test for `/binauth` producing a `session_end` event.

### FEAT-03 Session duration is never enforced on the network

- Priority: High
- Where: `common/fas-server`, agent config
- Subagent: `backend-service`

The central platform's `session_seconds` is written to sqlite but has no effect on the client's access. At levels 0 to 2 the `authdir` redirect cannot carry a session length. openNDS applies its own global `sessiontimeout`.

Build one of the following and document the choice:

- after a grant, call `ndsctl auth <mac> <minutes>` through a trait-backed command runner (the agent runs on the router, so this is local); or
- at expiry, call `ndsctl deauth <mac>` from the sweep job; or
- use level 3 or 4 with authmon, which accepts `sessionlength` and rate and quota limits (needs an https FAS, so not the local agent).

Acceptance: a test with a fake command runner shows the right `ndsctl` invocation for a 30 minute grant.

### FEAT-04 Central policy and landing redirect are ignored

- Priority: Medium
- Where: `common/central-client` (`fetch_policy` is never called), `common/fas-server/src/controllers/fas_controller.rs` (`redirect_url` is stored but the redirect always uses `originurl`)
- Subagent: `backend-service`

Build: send `redir = redirect_url` from the central decision when present (validate that it is `http` or `https`). Fetch the policy at start and every `heartbeat.interval_secs`, cache it in memory and in sqlite, and use it in place of `session.default_duration_secs` when the central platform is unreachable.

Acceptance: tests for the redirect choice and for offline use of the cached policy.

### FEAT-05 Config validation for the agent

- Priority: Medium (prerequisite for SEC-05 and SEC-06)
- Where: `common/agent-config`
- Subagent: `backend-service`

Build `Settings::validate()`, called from `parse`. Rules: non-empty `venue.id` (at most 64 characters, `^[A-Za-z0-9_-]+$`); `server.bind_addr` parses as `SocketAddr`; all intervals and timeouts are at least 1; `sync.batch_size` is 1 to 1,000; the HTTPS rule from SEC-06; `faskey` is required for secure levels. Add `ConfigError::Invalid { field, reason }`.

Acceptance: one test per rule in `common/agent-config/tests/settings.rs`.

### FEAT-06 OpenWrt build, packaging and service script

- Priority: High (the agent cannot be deployed today)
- Where: `apps/backend/agent`, `.github/workflows/continuous-integration.yml`, `devops/scripts`
- Subagents: `infrastructure`, `backend-service`

The GL-MT6000 is a MediaTek Filogic 830 (MT7986), an aarch64 Cortex-A53 device.

Build:

1. Cross-compile for `aarch64-unknown-linux-musl` (using `cross` or `cargo-zigbuild`), with release profile size settings for the agent (`opt-level = "z"` per package if the binary is too large for flash).
2. A procd init script (`apps/backend/agent/openwrt/ru5ty-gate-agent.init`) with respawn, which reads `/etc/ru5ty-gate/agent.toml`, and a sample openNDS UCI snippet (`fasport`, `faspath`, `fas_secure_enabled`, `faskey`). These files live under the agent app, not in a new top-level folder.
3. A CI job that builds the musl binary and uploads it as an artifact on tags.
4. The agent logs to stdout; set `procd_set_param stdout 1` and `stderr 1` in the init script so the output reaches `logread`.

Acceptance: the CI artifact runs `ru5ty-gate-agent --version` under `qemu-aarch64`.

### FEAT-07 Router clock sanity before granting sessions

- Priority: Medium
- Where: `common/session-store/src/unix_time.rs`, fas-server
- Subagent: `backend-service`

Routers without a battery-backed clock boot with a wrong time until NTP syncs. Sessions granted then get wrong `granted_at` and `expires_at`, and the sweep may delete live sessions or keep dead ones. Build a check: treat time as untrusted while `unix_now()` is before the build timestamp (embed it with `env!` from a `build.rs`, or use a constant updated at release). While untrusted, grant using the default duration but mark the session so the sweep recomputes expiry once time is valid.

### FEAT-08 Agent observability

- Priority: Low
- Subagent: `backend-service`

Add `pending_events` and `last_sync_ok_at` to `HeartbeatRequest` (agree the change with FEAT-01), and add a `healthcheck` subcommand to the agent binary like the other services (`ru5ty-gate-agent healthcheck` calls `/health` and exits 0 or 1). procd can use it.

### FEAT-09 Admin dashboard for venues, gateways and sessions

- Priority: Medium (after FEAT-01)
- Where: `apps/frontend/admin-web`
- Subagents: `frontend-page-builder`, `frontend-react`. Run `/ui-ux-pro-max` before and `/impeccable audit` after, per `CLAUDE.md`.

Pages: venues (CRUD), gateways per venue (create, rotate API key and show it once, last heartbeat, agent version, queue depth), live and historical sessions (filters by venue and date; hashed identifiers only, per SEC-08). Form validation as `CLAUDE.md` requires.

### FEAT-10 Splash and consent page

- Priority: Medium (out of v1 scope in the README, but needed for a real portal)
- Where: `apps/frontend/customer-web` or openNDS ThemeSpec
- Subagent: `frontend-nextjs` (captive pages are not indexed, so `/seo-optimization` does not apply)

A branded terms and consent page that runs before the grant, and records consent with the session (POPIA). Prav needs to decide whether this lives in openNDS (ThemeSpec, local, works offline) or in customer-web (central, needs walled-garden entries in openNDS).

### FEAT-11 End-to-end test against real openNDS request shapes

- Priority: High (it would have caught SEC-01)
- Where: `apps/backend/agent/tests/integration/`
- Subagent: `testing` (keep on Sonnet: this is a new test strategy, not a pattern copy)

Build fixtures from the openNDS reference scripts: level 1 base64 payloads with and without custom parameters, and the matching expected `tok` values. Run the real `Application` on an ephemeral port with a stub central server (local axum listener on `127.0.0.1:0`, as `common/heartbeat/tests/sync_task.rs` already does). Cover grant, deny, offline fail-open, offline fail-closed, sync after recovery and session end.

---

## Suggested order

1. SEC-10 (critical RCE in a dependency, quick to fix), then SEC-15 so it stays fixed.
2. FEAT-05, then SEC-05 and SEC-06 (config foundations).
3. SEC-01, then FEAT-11 (makes the agent work with real openNDS, with tests that prove it).
4. SEC-02, SEC-03, SEC-04, SEC-09, SEC-07, SEC-08 (harden the FAS endpoint).
5. SEC-11, SEC-12, SEC-13, SEC-14 (platform hardening).
6. FEAT-01 (central endpoints), then FEAT-02, FEAT-03, FEAT-04.
7. FEAT-06 (deployable build), FEAT-07, FEAT-08.
8. FEAT-09, FEAT-10 (UI).

## Decisions needed from Prav

Decisions taken while implementing (change them if you disagree):

- Fail-open stays the default (`allow_offline = true`). The agent only grants a redirect token derived from the `faskey`, validates every field, and keeps `/status` and `/binauth` on a separate admin listener, so a captive client cannot reach them. Switch to fail-closed if you would rather deny everyone during an outage.
- openNDS level 1 is the only supported level.
- The central API lives in `admin-api`, with device authentication through per-gateway API keys.
- The unused template services (`customer-api`, `schedule-api`, CMS, n8n) were kept.

Still open:

- Fail-open (`allow_offline = true`, the current default) or fail-closed when the central platform is down. Fail-open is friendlier, but combined with SEC-02 it grants everyone during an outage.
- Which openNDS level to standardise on: 1 (simplest, recommended for a local FAS) or 2.
- Where the splash and consent page lives (FEAT-10), and the POPIA retention period for session data (SEC-08).
- Whether the central API lives in `admin-api` or in a new device-facing service (FEAT-01).
- Whether to keep the template's `customer-api`, `schedule-api`, CMS and n8n apps, or remove the ones the portal does not need. Every unused service is attack surface and appears in the audit results.
- The platform findings (SEC-10 to SEC-15) also exist in the Zynkosi Tech Rust monorepo template. Decide whether to fix them there too, so new projects do not inherit them.

## Reproducing the agent findings

```bash
cargo build -p ru5ty-gate-agent
cat > /tmp/agent.toml <<'EOF'
[venue]
id = "v1"

[server]
bind_addr = "127.0.0.1:4019"
db_path = "/tmp/ru5ty-gate-review/sessions.db"

[central]
base_url = "http://127.0.0.1:1"
timeout_secs = 1
EOF
./target/debug/ru5ty-gate-agent --config /tmp/agent.toml &
```

Then run the `curl` commands listed under SEC-01, SEC-02 and SEC-03. For SEC-05, add `[heartbeat]` with `interval_secs = 0` to the config, start the agent, and look for `panicked at common/heartbeat/src/heartbeat_task.rs`.
