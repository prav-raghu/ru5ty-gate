# ru5ty-gate-database

SQLx migrations, models, repositories, pool creation and seed functions.

## Public API

connect, run_migrations, models, `Repository<E>` with the `Entity`, `SoftDeletable` and `Writable` traits, entity repositories (`UserRepository`), `*Record` row types, input structs (`PageRequest`, `UserListFilter`, `NewWebhookSubscription`, `WebhookSubscriptionChanges`), `DatabaseError` (including `is_unique_violation()`), seed_all, the `seed` binary

Everything is re-exported from `src/lib.rs`; consumers never import internal module paths.

## Use

```toml
ru5ty-gate-database = { version = "1.0.0", path = "../../../common/database" }
```

```rust
use ru5ty_gate_database::{PgPool, Repository, WebhookSubscription};

async fn find(pool: &PgPool, id: uuid::Uuid) -> Result<Option<WebhookSubscription>, ru5ty_gate_database::DatabaseError> {
    Repository::<WebhookSubscription>::find_by_id(pool, id).await
}
```

See [documentation/repository-layer.md](../../documentation/repository-layer.md).

## Test

```bash
cargo test -p ru5ty-gate-database
```
