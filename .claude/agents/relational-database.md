---
name: relational-database
description: Use when working with the database at the operations level — writing or modifying SQLx migrations, seeding data, reviewing naming conventions for tables/columns, or debugging database issues including query performance and connection problems on Azure PostgreSQL.
tools: Read, Write, Bash, Grep, Glob
model: inherit
---

## Stack

PostgreSQL (Azure-hosted or Coolify-managed), SQLx 0.9 (runtime-checked queries, `FromRow` models), migrations in `common/database/migrations`. Single shared schema across all services with clear domain boundaries. Connection is `DATABASE_URL` only.

## Naming conventions — strictly enforced

| Element | Convention | Example |
|---|---|---|
| Database name | `snake_case` | `ru5ty_gate` |
| Table names | `snake_case` plural | `users`, `appointment_slots` |
| Column names | `snake_case` | `user_id`, `created_at`, `first_name` |
| Rust models | `PascalCase` singular | `UserProfile` |
| Constraints | `{table}_{column}_key`, `idx_{table}_{columns}` | `users_email_key` |

```sql
CREATE TABLE user_profiles (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    first_name VARCHAR(100) NOT NULL,
    last_name  VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Commands

```bash
cargo build -p ru5ty-gate-database
cargo test -p ru5ty-gate-database
cargo run --bin seed            # developer only: inserts dev seed data
./devops/scripts/migrate.sh     # developer only: applies migrations
```

Agents do not run migrations or seeds against a real database.

## Service domain boundaries

| Service | Domain tables |
|---|---|
| `customer-api` | customer-facing entities |
| `admin-api` | admin/management entities, `system_bootstrap` |
| `schedule-api` | scheduling, jobs, webhook deliveries |
| `api-gateway` | none — it does not touch the database |

Do not cross domain boundaries in queries — use service-to-service communication instead. Shared tables (`users`, `roles`, `webhook_*`) are accessed through each service's own service layer.

## Migration rules

Never modify an existing migration — always create a new one. Names must be descriptive: `add_user_profile_table`, `add_index_to_orders_user_id`. Every schema change requires a migration. Test on dev before staging or production. See the `database-migrations` agent for zero-downtime patterns.

## Query rules

- Queries live in `common/database/src/repositories/` (generic `Repository::<E>` plus `{Entity}Repository`), not in services
- Static SQL strings with bound parameters; never `format!` user input into SQL
- Dynamic filters use `QueryBuilder` with `push_bind`
- `fetch_optional` for lookups that may miss, `fetch_one` only when a row is guaranteed (inserts with `RETURNING`)
- Multi-statement writes use `pool.begin()` and an explicit `commit()`; dropping the transaction rolls back
- Always list columns; never `SELECT *` into a response type
- Map unique-violation errors to `AppError::Conflict`: `DatabaseError::is_unique_violation()` for repository errors (`error.as_database_error().is_some_and(|database| database.is_unique_violation())` on a raw `sqlx::Error`)

## Azure considerations

Avoid database transactions where a single statement works — Azure PostgreSQL is prone to deadlocks under transaction-heavy workloads. No retry logic at the DB layer — handle retries at the service layer instead. For batch operations, filter to only records with genuine changes to avoid oversized queries. Use `UNNEST` arrays or `JOIN`/`UNION ALL` patterns for batch updates rather than `CASE`-based queries with excessive bound parameters. Pool size is `DATABASE_CONNECTION_LIMIT` and acquire timeout `DATABASE_POOL_TIMEOUT`; keep the sum of pool sizes across replicas below the server's `max_connections`.

## Optional: MongoDB

If a service needs NoSQL for high-throughput document storage, set it up as a separate connection in that service. Never mix relational and document storage in the same domain. Configure connection via environment variables only.

## Environment variables

```env
DATABASE_URL="postgresql://user:password@host:5432/dbname"
DATABASE_CONNECTION_LIMIT=10
DATABASE_POOL_TIMEOUT=10
```

Never hardcode connection strings.

## Seeding

Idempotent seed functions live in `common/database/src/seed.rs` (`seed_roles`, `seed_user_statuses`, `seed_admin`, `seed_all`) and are called by `src/bin/seed.rs` and by tests. Use `INSERT ... ON CONFLICT DO NOTHING` so re-running is safe, and read credentials such as `ADMIN_EMAIL` and `ADMIN_PASSWORD` from the environment — never hardcode them.
