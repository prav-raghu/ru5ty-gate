# ru5ty-gate-sms

SMSPortal client.

## Public API

SmsService, SmsSender, SmsConfig, to_e164

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-sms = { version = "1.0.0", path = "../../../common/sms" }
```

```rust
use ru5ty_gate_sms;
```

## Test

```bash
cargo test -p ru5ty-gate-sms
```
