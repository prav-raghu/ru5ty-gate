---
description: Write unit or integration tests for a service, controller, or endpoint
argument-hint: <what to test, e.g. "unit tests for ProductService" or "integration tests for order endpoints">
---

Use the testing subagent to write tests for: $ARGUMENTS

Match the structure of the nearest existing test file of the same kind. Service tests use `#[sqlx::test]` against a real Postgres; replace external services (email, HTTP receivers) with trait implementations or local listeners. Run `cargo test -p <crate>` and `cargo clippy --workspace --all-targets -- -D warnings` afterward and report pass/fail.
