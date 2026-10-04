---
name: database-migrations
description: Use when writing or modifying SQLx migrations, deciding on safe zero-downtime migration patterns, planning a backfill, or reviewing the migration execution strategy for CI/CD. Trigger on "migration", "add a column safely", "rename a column/table", or "backfill".
tools: Read, Edit, Write, Bash, Grep, Glob
model: inherit
---

## How migrations work here

Migrations are plain SQL files in `common/database/migrations/` named `YYYYMMDDHHMMSS_description.sql`. They are embedded into the binary with `sqlx::migrate!` and applied by `run_migrations` in `common/database/src/pool.rs`. The `migrate` subcommand of the `admin-api` binary is the one command that applies them; the compose `migrate` one-shot service runs it using the `admin-api` image.

| Command | When | Who runs it |
|---------|------|-------------|
| `./devops/scripts/migrate.sh` (runs the `admin-api` binary with `migrate`) | Local development against a dev database | The developer |
| The `migrate` compose service (`command: ["migrate"]` on the `admin-api` image) | Staging and production, before the APIs start | Coolify, on every deploy |

Agents never run either — the developer does. `.claude/settings.json` denies them.

`common/database/build.rs` emits `cargo:rerun-if-changed=migrations`, so adding a file triggers a rebuild and the new migration is embedded.

## Migration file discipline

Migration files are immutable once merged to `main` — never edit one that has been applied anywhere (SQLx checksums applied files and refuses to start on a mismatch). Each migration is atomic, one logical change. Names are descriptive: `add_products_table`, `add_slug_to_products`, `drop_legacy_sessions`. Migrations stay backward compatible with the previous code version — the running app must survive the migration before the new app version deploys.

Every new table follows `rules/database.md`: the six base columns (or a documented exception), foreign-key indexes, the cursor pagination index.

## Zero-downtime patterns

Adding a column (safe): `ALTER TABLE products ADD COLUMN slug VARCHAR(255);` — the existing app version ignores the new column. Rust queries with explicit column lists are unaffected.

Making a column NOT NULL (two migrations): never add `NOT NULL` without a default on a table with existing rows in one step. Migration 1 adds it nullable. Run a backfill. Migration 2 (after the backfill) runs `ALTER TABLE products ALTER COLUMN slug SET NOT NULL;`.

Renaming a column (never directly): direct renames break the running app immediately. Use expand-contract: Phase 1 add the new column and copy data (`ALTER TABLE products ADD COLUMN new_name VARCHAR(255); UPDATE products SET new_name = old_name;`); Phase 2 deploy the app version reading `new_name`; Phase 3 drop the old column in a follow-up migration once confirmed.

Renaming a table: same pattern — add new table, dual-write, migrate reads, remove old table. Never a one-step rename.

Dropping a column or table: only after confirming no running code references it. Deploy a version removing all references first, then drop in the next migration.

Adding an index on a large table: standard `CREATE INDEX` takes a lock that blocks writes. SQLx runs each migration in a transaction by default, and `CREATE INDEX CONCURRENTLY` cannot run inside one. For tables over roughly 1M rows, put the concurrent index in its own migration whose first line is `-- no-transaction`, and write `CREATE INDEX CONCURRENTLY IF NOT EXISTS ...`.

## Backfill pattern

1. Add the column as nullable in the migration
2. Add a job `backfill_{column}` in `schedule-api` (a `CronSchedulerService` job)
3. Process rows in batches of 500 with keyset iteration on `id`
4. Make the job idempotent — safe to re-run if it fails midway
5. Follow-up migration makes the column `NOT NULL` after the backfill completes

```rust
pub async fn backfill_slugs(&self) -> Result<u64, AppError> {
    let mut last_id: Option<Uuid> = None;
    let mut updated = 0_u64;
    loop {
        let batch = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, name FROM products WHERE slug IS NULL AND ($1::uuid IS NULL OR id > $1) \
             ORDER BY id LIMIT 500",
        )
        .bind(last_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((last, _)) = batch.last().cloned() else { break };
        let mut transaction = self.pool.begin().await.map_err(AppError::internal)?;
        for (id, name) in &batch {
            sqlx::query("UPDATE products SET slug = $2, updated_at = NOW() WHERE id = $1")
                .bind(id)
                .bind(generate_slug(name))
                .execute(&mut *transaction)
                .await
                .map_err(AppError::internal)?;
        }
        transaction.commit().await.map_err(AppError::internal)?;
        updated += batch.len() as u64;
        last_id = Some(last);
    }
    Ok(updated)
}
```

## Migration execution strategy for this project

This project deploys via Coolify on a self-hosted VPS, not Kubernetes. The root `docker-compose.yaml` has a `migrate` one-shot service that runs the `admin-api` image's `migrate` subcommand and exits; `admin-api`, `customer-api` and `schedule-api` declare `depends_on: migrate: condition: service_completed_successfully`, so no API starts against an unmigrated schema. Never run migrations inside an API's normal startup — multiple replicas would race. See `deployment-coolify` for the full mechanism.

## Seed and lookup data

Reference data the application cannot run without (roles, user statuses, lookup rows) is inserted by a migration with `ON CONFLICT DO NOTHING`, so a fresh database is usable immediately after the migrate step. Development-only data lives in the `seed` binary (`common/database/src/bin/seed.rs`).

## Testing migrations

`#[sqlx::test(migrations = "../../../common/database/migrations")]` applies the full migration set to a fresh database for every test, so a broken migration fails the whole suite. Add a test in `common/database/tests` when a migration contains data changes.

## CI/CD migration checklist

`cargo test --workspace` passes against Postgres 16 (this applies every migration). A migration file exists for every schema change — no direct DB edits. New `NOT NULL` columns have a default or are added nullable with a backfill plan. Large table index additions use `CREATE INDEX CONCURRENTLY` in a `-- no-transaction` migration. Column/table renames follow expand-contract, never a direct rename. The compose `migrate` service runs before any API starts.
