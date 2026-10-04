# OpenWrt deployment

Files for installing the agent on a GL-iNet GL-MT6000 (MediaTek Filogic 830, aarch64) running OpenWrt with openNDS.

## Files

| File | Install to | Purpose |
|---|---|---|
| `ru5ty-gate-agent.init` | `/etc/init.d/ru5ty-gate-agent` (mode 755) | procd service with respawn, logs to `logread` |
| `custombinauth.sh` | `/usr/lib/opennds/custombinauth.sh` (mode 755) | tells the agent when openNDS deauthenticates a client |
| `opennds.uci.example` | merge into `/etc/config/opennds` | FAS level 1 settings and the pre-auth firewall rule |
| `../config/agent.router.example.toml` | `/etc/ru5ty-gate/agent.toml` (mode 600, owner root) | agent settings |

`custombinauth.sh` replaces the stock file. If the router's copy already holds your own logic, append the `case "$method"` block instead of overwriting it.

## Build

The CI job `agent-openwrt` builds the binary for `aarch64-unknown-linux-musl` with `cross` and uploads it as an artifact. The same build by hand:

```bash
cargo install cross --locked
cross build --release --target aarch64-unknown-linux-musl -p ru5ty-gate-agent
```

The result is `target/aarch64-unknown-linux-musl/release/ru5ty-gate-agent`, a statically linked binary with sqlite bundled in.

## Install

```bash
scp target/aarch64-unknown-linux-musl/release/ru5ty-gate-agent root@192.168.1.1:/usr/bin/
scp apps/backend/agent/openwrt/ru5ty-gate-agent.init root@192.168.1.1:/etc/init.d/ru5ty-gate-agent
scp apps/backend/agent/openwrt/custombinauth.sh root@192.168.1.1:/usr/lib/opennds/custombinauth.sh
ssh root@192.168.1.1 'mkdir -p /etc/ru5ty-gate && chmod 700 /etc/ru5ty-gate'
scp apps/backend/agent/config/agent.router.example.toml root@192.168.1.1:/etc/ru5ty-gate/agent.toml
ssh root@192.168.1.1 'chmod 600 /etc/ru5ty-gate/agent.toml && chmod +x /etc/init.d/ru5ty-gate-agent /usr/lib/opennds/custombinauth.sh'
```

Edit `/etc/ru5ty-gate/agent.toml` on the router and replace every `REPLACE_ME`, merge `opennds.uci.example` into `/etc/config/opennds`, then:

```bash
service ru5ty-gate-agent enable
service ru5ty-gate-agent start
service opennds restart
logread -e ru5ty-gate-agent
ru5ty-gate-agent --config /etc/ru5ty-gate/agent.toml healthcheck && echo healthy
```

## Verify on the device

Nothing in this directory has been run on real hardware. Check these on the first router:

- A client is redirected to the agent, gets a 302 back to `http://<router>/opennds_auth/?tok=...`, and gains internet access.
- `curl http://127.0.0.1:2081/status` on the router shows the session and `"clock_trusted":true`.
- Running `ndsctl deauth <mac>` makes the session disappear from `/status` and queues a `session_end` event.
- A session that reaches its granted time is deauthenticated by the agent (`logread` shows no ndsctl errors).
- The `users_to_router` rule is accepted by your openNDS version. Older releases name this ruleset differently.
- The binary size fits in flash alongside the rest of the firmware.
