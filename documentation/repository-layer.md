# Repository layer

All SQL lives in `common/database`. Services call repositories and never contain queries. There is no ORM: the layer is thin sqlx code with one generic repository for plain CRUD and small entity repositories for everything else.

## Layout

```text
common/database/src/
├── models/          one {entity}.rs per table (FromRow), implements Entity
├── repositories/    Repository<E>, Entity, SoftDeletable, Writable, {entity}_repository.rs
├── records/         *Record structs: row shapes returned by repository queries
└── inputs/          structs passed in: Writable inserts and updates, filters, PageRequest
```

Everything is re-exported from `lib.rs`; services import from `ru5ty_gate_database`.

## Generic repository

`Repository::<E>` works for any model that implements `Entity`. Every function is generic over `sqlx::PgExecutor`, so it accepts `&PgPool` or `&mut *transaction`.

| Function | Returns |
|----------|---------|
| `find_by_id(executor, id)` | `Option<E>` |
| `list(executor, PageRequest)` | `Vec<E>` ordered by `Entity::ORDER_BY` (default `created_at DESC, id`) |
| `count(executor)` | `i64` |
| `exists(executor, id)` | `bool` |
| `insert(executor, &W)` | the inserted `E` (`RETURNING *`) |
| `update_by_id(executor, id, &W)` | `Option<E>`, replacing every column `W` lists |
| `delete_by_id(executor, id)` | `bool` (a row was removed) |
| `deactivate(executor, id, modified_by)` | `bool`; only for `SoftDeletable` models |

`PageRequest::new(limit, offset)` clamps the limit to 1-100 (default 20) and the offset to 0 or more.

### Making a model usable

```rust
impl Entity for WebhookSubscription {
    const TABLE: &'static str = "webhook_subscriptions";
}

impl SoftDeletable for WebhookSubscription {}
```

`SoftDeletable` is opt-in because not every table has `is_active` and `modified_by` (`webhook_deliveries` does not).

Table names come only from `Entity::TABLE`, a compile-time constant. Every value is bound as a parameter.

### Writing rows

A write shape is a struct implementing `Writable`. It names its columns and binds the values in the same order:

```rust
pub struct NewWebhookSubscription {
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub retry_count: i32,
    pub timeout_seconds: i32,
    pub created_by: String,
}

impl Writable for NewWebhookSubscription {
    type Entity = WebhookSubscription;
    const COLUMNS: &'static [&'static str] = &[
        "url", "secret", "events", "retry_count", "timeout_seconds", "created_by", "modified_by",
    ];

    fn bind_values(&self, values: &mut Separated<'_, Postgres, &'static str>) {
        values
            .push_bind(&self.url)
            .push_bind(&self.secret)
            .push_bind(&self.events)
            .push_bind(self.retry_count)
            .push_bind(self.timeout_seconds)
            .push_bind(&self.created_by)
            .push_bind(&self.created_by);
    }
}
```

Using it from a service:

```rust
let subscription = Repository::<WebhookSubscription>::insert(&self.pool, &new_subscription)
    .await
    .map_err(AppError::internal)?;
```

`insert` and `update_by_id` use `RETURNING *`, so the model must have a field for every column of its table. `updated_at` is set by the database trigger, not by the repository.

## Entity repositories

Anything that does not fit the generic functions (joins, filters, ownership checks, partial updates, cursor pagination) goes in a unit struct named after the entity, with associated functions generic over `PgExecutor`:

```rust
pub struct UserRepository;

impl UserRepository {
    pub async fn find_active_summary<'e, E>(
        executor: E,
        user_id: Uuid,
    ) -> Result<Option<UserSummaryRecord>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let record = sqlx::query_as::<_, UserSummaryRecord>(
            "SELECT id, username, age, last_seen FROM users WHERE id = $1 AND is_active = TRUE",
        )
        .bind(user_id)
        .fetch_optional(executor)
        .await?;
        Ok(record)
    }
}
```

Rules for entity repositories:

- One struct per file, named `{entity}_repository.rs`, re-exported from `repositories/mod.rs` and `lib.rs`
- Static SQL strings with bound parameters; dynamic filters use `QueryBuilder` with `push_bind`
- Return `DatabaseError`, never `AppError`; the database crate does not know about HTTP
- Row shapes that are not the full model are `*Record` structs in `records/`; inputs with more than a couple of arguments are structs in `inputs/` (for example `UserListFilter`)

## In a service

```rust
pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<UserSummary>, AppError> {
    let record = UserRepository::find_active_summary(&self.pool, user_id)
        .await
        .map_err(AppError::internal)?;
    Ok(record.map(UserSummary::from))
}
```

The DTO is plain data and gets an `impl From<UserSummaryRecord> for UserSummary`. A unique-constraint violation becomes a 409 with `DatabaseError::is_unique_violation()`:

```rust
.map_err(|error| {
    if error.is_unique_violation() {
        AppError::Conflict("Product already exists".to_owned())
    } else {
        AppError::internal(error)
    }
})
```

## Transactions

Pass the transaction as the executor. The same repository functions then run inside it:

```rust
let mut transaction = pool.begin().await?;
Repository::<WebhookSubscription>::insert(&mut *transaction, &new_subscription).await?;
transaction.commit().await?;
```

Dropping the transaction without `commit()` rolls it back.

## Adding a new entity

1. Add the migration and the model (see `.claude/rules/database.md`)
2. `impl Entity for Model` and, if the table has `is_active` and `modified_by`, `impl SoftDeletable for Model {}`
3. Add a `Writable` input per write shape you need
4. Add `{entity}_repository.rs` and any `*Record` types only for queries the generic layer cannot express
5. Re-export the new items from `lib.rs`
6. Add tests in `common/database/tests/` with `#[sqlx::test(migrations = "./migrations")]` (see `tests/repository.rs`)

## Testing

Repository tests use a real database. `#[sqlx::test]` creates an isolated database per test, applies the migrations and drops it afterwards, so they need `DATABASE_URL` pointing at a role that can create databases:

```bash
cargo test -p ru5ty-gate-database
```

Service tests in `apps/backend/*/tests/services/` exercise repositories through the services, also against a real database.

## Migration status

The layer is new. Migrated so far:

- customer-api `UserService` (all four queries)
- customer-api `WebhookSubscriptionService::create_subscription` (generic `insert`)

Still containing inline SQL, to be moved when each service is next changed: admin-api (user, auth, bootstrap, reporting and batch services), the remaining customer-api auth and webhook subscription queries, the webhook delivery service in `common/webhooks`, and `seed.rs`.

## sqlx 0.9 note

In sqlx 0.9, `QueryBuilder` and `Separated` carry no argument lifetime because bound values are encoded as they are pushed. That is why `Writable::bind_values` takes `&self` with no extra lifetime parameter.
