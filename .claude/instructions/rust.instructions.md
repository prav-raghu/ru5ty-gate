---
applyTo: "**/*.rs"
description: "Rust build validation — must pass before any Rust task is considered complete"
---

# Rust Build Validation

Every Rust crate in this monorepo must pass the full gate before any task is considered complete. This is non-negotiable.

## Required checks before marking any task as done

Run from the monorepo root before completing any task that touches Rust files:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ru5ty-gate-<crate>
```

Or for the full check, the same gates the root scripts run:

```bash
pnpm lint:rust
pnpm typecheck:rust
pnpm test:rust
```

Tests that use the database or Redis need `DATABASE_URL` (a Postgres role that can create databases) and, optionally, `TEST_REDIS_URL`.

If any command produces errors or warnings, the task is NOT complete. Fix everything before finishing.

## Rules

- `cargo check` passing is not enough; clippy pedantic with `-D warnings` is the lint gate
- Never use `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` or `dbg!` in production code
- Never add `#[allow(...)]` to production code; test files may allow `unwrap_used`, `expect_used` and `panic` at the top of the file
- Never cast with `as` to silence a type error — convert with `try_from` and handle the failure
- No comments in code
- Match the existing style: one item per file, `mod.rs` re-exports, associated async functions on unit-struct controllers
