---
paths:
  - "**/tests/**/*.rs"
  - "**/*.test.ts"
  - "**/*.test.tsx"
---

# Testing Rules

You are writing tests. These rules apply to all test files. Backend tests are Rust; frontend tests keep the Vitest/Jest conventions of their own app.

## Backend test types and where they live

Each service crate has:

- `tests/services/` — service-layer tests against a real database (`#[sqlx::test]`). **Required for every service.**
- `tests/integration/` — full HTTP tests that build the `Router` and call it with `tower::ServiceExt::oneshot`
- `tests/unit/` — pure tests for config loading and request validation (no database)
- `tests/common/mod.rs` — shared helpers: `UserFactory`-style builders, `RecordingEmailSender`, `test_config()`, `call()` and `login_token()`

Each test directory has a `main.rs` that declares its modules and includes `common` with `#[path = "../common/mod.rs"] mod common;`, so every directory compiles as one test binary. Common crates keep unit tests inline in `#[cfg(test)] mod tests` or in their own `tests/` directory.

## Real database per test — no mocks for SQL

`#[sqlx::test(migrations = "../../../common/database/migrations")]` creates an isolated database per test, applies the migrations and drops it afterwards, so tests never share rows and never need manual cleanup.

```rust
#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn lists_active_users_excluding_the_caller(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.user;
    let me = UserFactory::new(&pool, "Me Myself").create().await;

    let users = service.get_users(&UserFilters::default(), Some(me)).await.unwrap();

    assert!(users.is_empty());
}
```

External services are replaced by trait implementations, never by mocking SQL: `EmailSender` is replaced by `RecordingEmailSender`, and webhook endpoints by a local Axum listener bound to `127.0.0.1:0`.

## Redis

`live_redis()` connects to `TEST_REDIS_URL` when set and falls back to `RedisService::disabled()`. Tests that need Redis behaviour (blacklist, lockout, `minIat`) must use unique user ids or key prefixes — never flush the database.

## Environment variables

```
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/postgres
TEST_REDIS_URL=redis://127.0.0.1:6379
```

`#[sqlx::test]` reads `DATABASE_URL` and needs a role that can create databases. CI provides Postgres 16 and Redis 7 service containers.

## Lint exceptions for tests

The workspace denies `unwrap`, `expect` and `panic`. Test files opt out explicitly at the top of the file:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
```

Production code never uses this attribute.

## Required coverage per service method

| Method | Must test |
|---|---|
| `find_by_id` | found, not found, cached path when a cache is used |
| `list` | returns paginated data, applies each filter, respects limit/offset |
| `create` | creates the record, rejects duplicates with `Conflict`, sets `created_by` |
| `update` | updates the record, sets `modified_by` and `updated_at`, not found |
| `delete` (soft) | sets `is_active = FALSE`, not found |
| Optimistic lock | returns `Conflict` when the version does not match |

## Required coverage per route

| Scenario | Expected |
|---|---|
| Missing auth token | 401 |
| Wrong role/permission | 403 |
| Invalid body | 400 with field-level `errors` |
| Unknown field in body | 400 |
| Not found | 404 |
| Happy path GET | 200 + `{ isSuccessful: true, data }` |
| Happy path POST | 201 + `{ isSuccessful: true, data }` |
| Duplicate | 409 |

## Naming pattern

Test functions are plain-English sentences in `snake_case`: `{subject}_{does_action}_{when_condition}`.

```rust
async fn find_by_id_returns_none_when_the_user_is_inactive(pool: PgPool)
async fn login_locks_the_account_after_repeated_failures(pool: PgPool)
```

## Commands

```bash
cargo test -p ru5ty-gate-<service>
cargo test --workspace
```

Coverage for Sonar is produced with `cargo llvm-cov --workspace --lcov` in CI; there is no local threshold gate.

## Frontend tests

Frontend unit tests live beside the code in `apps/frontend/*` and `apps/mobile/*` and follow the conventions of that app's own test runner. They are not covered by the Rust commands above.
