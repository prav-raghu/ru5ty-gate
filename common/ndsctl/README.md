# ru5ty-gate-ndsctl

Runs the local openNDS `ndsctl` command for the captive portal agent.

## Public API

Ndsctl (disabled, with_program, is_enabled, deauth), NdsctlError

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Notes

- `deauth` runs `<program> deauth <mac>` without a shell, with a 5 second timeout, and only after checking that the argument is a MAC address (six colon-separated hex octets). Anything else returns `NdsctlError::InvalidMac`.
- `Ndsctl::disabled()` makes every call a no-op, which is the development default.
- The agent calls it when a session expires, so a session shorter than openNDS's `sessiontimeout` is enforced on the network.

## Use

```toml
ru5ty-gate-ndsctl = { version = "1.0.0", path = "../../../common/ndsctl" }
```

```rust
use ru5ty_gate_ndsctl::Ndsctl;
```

## Test

```bash
cargo test -p ru5ty-gate-ndsctl
```
