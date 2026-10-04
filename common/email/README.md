# ru5ty-gate-email

Email sending with HTML templates (Mailtrap).

## Public API

EmailSender trait, EmailService, EmailConfig, render_template

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-email = { version = "1.0.0", path = "../../../common/email" }
```

```rust
use ru5ty_gate_email;
```

## Test

```bash
cargo test -p ru5ty-gate-email
```
