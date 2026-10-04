---
paths:
  - "common/database/**"
---

# Database Rules (SQLx + PostgreSQL)

You are working on the shared database crate. Every change here affects all services.

## Layout

```
common/database/
├── Cargo.toml
├── build.rs                     re-runs the build when migrations change
├── migrations/                  timestamped *.sql, embedded with sqlx::migrate!
├── src/
│   ├── lib.rs
│   ├── database_config.rs       DATABASE_URL, pool size, acquire timeout
│   ├── database_error.rs
│   ├── pool.rs                  connect, run_migrations
│   ├── models/                  one {entity}.rs per table, deriving FromRow and implementing Entity
│   ├── repositories/            generic Repository<E>, the Entity/SoftDeletable/Writable traits, {entity}_repository.rs
│   ├── records/                 *Record row projections returned by repository queries
│   ├── inputs/                  structs passed into repositories: Writable inserts/updates, filters, PageRequest
│   ├── seed.rs                  seed_roles, seed_user_statuses, seed_admin, seed_all
│   └── bin/seed.rs              seed binary
└── .env.example
```

## Every business table must have all six base columns

```sql
id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
is_active   BOOLEAN      NOT NULL DEFAULT TRUE,
created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
created_by  VARCHAR(255) NOT NULL DEFAULT 'SYSTEM',
modified_by VARCHAR(255) NOT NULL DEFAULT 'SYSTEM'
```

`updated_at` is maintained by the database: every table has a `{table}_set_updated_at` trigger (function `set_updated_at()` from the initial schema), so an `UPDATE` does not need to set it. Give every new table the same trigger. `created_by`/`modified_by` hold the acting user's id (or `'SYSTEM'` for non-interactive writes) — always set explicitly by the service layer on create/update, never left to the DB default in a user-initiated request. A table with one pair and not the other is a bug, not a style choice.

### Two sanctioned exceptions — narrow, and only for these shapes

1. **Truly append-only, never updated** (a pure event/audit-trail row): drop `updated_at`, `created_by`, `modified_by`. Still gets `id`, `created_at`. Join tables and lookup tables fall in this category too.

2. **System-owned, never human-actioned, but internally mutated** (e.g. a background worker updates retry/status fields, no user-facing endpoint ever writes to it): keep `updated_at`, but `created_by`/`modified_by` may be dropped. See `webhook_deliveries`: a delivery worker updates `status`/`attempt_count`/`next_retry_at`, so it keeps `updated_at`, but no per-request actor writes to it.

Anything reachable from a user-facing request keeps the full six. See `webhook_subscriptions`: registered and mutated through admin endpoints, so it keeps both `created_by` and `modified_by`.

## Repository layer

Every table with a model gets repository support:

1. The model in `models/{entity}.rs` derives `FromRow` and has `impl Entity for {Model} { const TABLE: &'static str = "{table}"; }`. `Entity::ORDER_BY` defaults to `created_at DESC, id`, which matches the pagination index above
2. Add `impl SoftDeletable for {Model} {}` only when the table has `is_active` and `modified_by` (`webhook_deliveries` has neither)
3. Each write shape is a struct implementing `Writable` (`NewWebhookSubscription` for inserts, `WebhookSubscriptionChanges` for full-column updates). It lists its `COLUMNS` and binds values in the same order. Partial updates, ownership-scoped writes and anything else that does not fit go in a `{Entity}Repository`
4. The generic `insert` and `update_by_id` use `RETURNING *`, so the model needs a field for every column of its table
5. Tests live in `common/database/tests/` and use `#[sqlx::test(migrations = "./migrations")]`; see `tests/repository.rs`

Details and examples: `documentation/repository-layer.md`.

## Naming

- Tables: `snake_case` plural
- Columns: `snake_case`
- Foreign keys: `{related_singular}_id`
- Rust models: `PascalCase` singular struct in its own file, e.g. `models/webhook_subscription.rs`

## Column types

| Data | SQL type | Rust type |
|---|---|---|
| Primary key | `UUID DEFAULT gen_random_uuid()` | `Uuid` |
| Money / currency | `NUMERIC(10, 2)` | `Decimal` |
| Short text | `VARCHAR(255)` | `String` |
| Long text | `TEXT` | `String` |
| Status / category | `VARCHAR` with a lookup table, or a Postgres enum | `String` / enum with `sqlx::Type` |
| Timestamps | `TIMESTAMPTZ` | `DateTime<Utc>` |
| Optional | nullable column | `Option<T>` |

## Relations

Declare foreign keys explicitly. Use `ON DELETE CASCADE` for child records, `ON DELETE SET NULL` for optional references. Always add an index on every foreign key column.

```sql
CREATE TABLE order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders (id) ON DELETE CASCADE
);
CREATE INDEX idx_order_items_order_id ON order_items (order_id);
```

## Every table needs a cursor pagination index

```sql
CREATE INDEX idx_{table}_created_at_id ON {table} (created_at DESC, id);
```

## Composite indexes for common query patterns

```sql
CREATE INDEX idx_products_category_active_created ON products (category_id, is_active, created_at DESC);
```

## Optimistic locking

For entities with concurrent write risk (orders, inventory, cart, payments) add `version INTEGER NOT NULL DEFAULT 1` and update with `WHERE id = $1 AND version = $2`, incrementing `version`. Zero rows affected means a conflict — return `AppError::Conflict`.

## Idempotency

For write-heavy transactional entities add `idempotency_key VARCHAR(255) UNIQUE` and return the existing row on a duplicate key.

## Migrations

Files live in `common/database/migrations/` named `YYYYMMDDHHMMSS_description.sql` and are embedded with `sqlx::migrate!`. They are applied by `admin-api migrate` (the compose `migrate` service) — never by application startup. Locally, run `cargo run --bin admin-api -- migrate` from `apps/backend/admin-api` (VS Code task `🗄️ DB: Migrate`); that folder's `.env` supplies `DATABASE_URL`.

- Migrations are forward-only and immutable once merged; fix mistakes with a new migration
- Seed reference data that production needs (roles, statuses, lookup rows) in a migration so a fresh database is usable after `migrate`
- `build.rs` makes Cargo rebuild when a migration is added

## Seed data

```rust
pub async fn seed_roles(pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::query("INSERT INTO roles (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
        .bind("Admin")
        .execute(pool)
        .await?;
    Ok(())
}
```

Seed functions are idempotent (`ON CONFLICT DO NOTHING`) and called from `src/bin/seed.rs` and from tests.

## After every migration or model change — required commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p ru5ty-gate-database
```

## Never

- Never run `sqlx migrate`, `admin-api migrate` or any command that applies migrations — the developer runs them
- Never edit a migration that has already been merged
- Never hardcode connection strings — `DATABASE_URL` only
- Never build SQL by string interpolation of user input
- Never add a column without a `NOT NULL`/default decision made on purpose
