use std::marker::PhantomData;

use sqlx::{PgExecutor, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::database_error::DatabaseError;
use crate::inputs::PageRequest;
use crate::repositories::{Entity, SoftDeletable, Writable};

pub struct Repository<E> {
    entity: PhantomData<E>,
}

impl<E: Entity> Repository<E> {
    pub async fn find_by_id<'e, X>(executor: X, id: Uuid) -> Result<Option<E>, DatabaseError>
    where
        X: PgExecutor<'e>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT * FROM ");
        builder.push(E::TABLE).push(" WHERE id = ").push_bind(id);
        let entity = builder
            .build_query_as::<E>()
            .fetch_optional(executor)
            .await?;
        Ok(entity)
    }

    pub async fn list<'e, X>(executor: X, page: PageRequest) -> Result<Vec<E>, DatabaseError>
    where
        X: PgExecutor<'e>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT * FROM ");
        builder
            .push(E::TABLE)
            .push(" ORDER BY ")
            .push(E::ORDER_BY)
            .push(" LIMIT ")
            .push_bind(page.limit)
            .push(" OFFSET ")
            .push_bind(page.offset);
        let entities = builder.build_query_as::<E>().fetch_all(executor).await?;
        Ok(entities)
    }

    pub async fn count<'e, X>(executor: X) -> Result<i64, DatabaseError>
    where
        X: PgExecutor<'e>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM ");
        builder.push(E::TABLE);
        let total = builder
            .build_query_scalar::<i64>()
            .fetch_one(executor)
            .await?;
        Ok(total)
    }

    pub async fn exists<'e, X>(executor: X, id: Uuid) -> Result<bool, DatabaseError>
    where
        X: PgExecutor<'e>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT EXISTS (SELECT 1 FROM ");
        builder
            .push(E::TABLE)
            .push(" WHERE id = ")
            .push_bind(id)
            .push(")");
        let found = builder
            .build_query_scalar::<bool>()
            .fetch_one(executor)
            .await?;
        Ok(found)
    }

    pub async fn insert<'e, X, W>(executor: X, values: &W) -> Result<E, DatabaseError>
    where
        X: PgExecutor<'e>,
        W: Writable<Entity = E>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("INSERT INTO ");
        builder
            .push(E::TABLE)
            .push(" (")
            .push(W::COLUMNS.join(", "))
            .push(") VALUES (");
        values.bind_values(&mut builder.separated(", "));
        builder.push(") RETURNING *");
        let entity = builder.build_query_as::<E>().fetch_one(executor).await?;
        Ok(entity)
    }

    pub async fn update_by_id<'e, X, W>(
        executor: X,
        id: Uuid,
        values: &W,
    ) -> Result<Option<E>, DatabaseError>
    where
        X: PgExecutor<'e>,
        W: Writable<Entity = E>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("UPDATE ");
        builder
            .push(E::TABLE)
            .push(" SET (")
            .push(W::COLUMNS.join(", "))
            .push(") = ROW(");
        values.bind_values(&mut builder.separated(", "));
        builder
            .push(") WHERE id = ")
            .push_bind(id)
            .push(" RETURNING *");
        let entity = builder
            .build_query_as::<E>()
            .fetch_optional(executor)
            .await?;
        Ok(entity)
    }

    pub async fn delete_by_id<'e, X>(executor: X, id: Uuid) -> Result<bool, DatabaseError>
    where
        X: PgExecutor<'e>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("DELETE FROM ");
        builder.push(E::TABLE).push(" WHERE id = ").push_bind(id);
        let result = builder.build().execute(executor).await?;
        Ok(result.rows_affected() > 0)
    }
}

impl<E: SoftDeletable> Repository<E> {
    pub async fn deactivate<'e, X>(
        executor: X,
        id: Uuid,
        modified_by: &str,
    ) -> Result<bool, DatabaseError>
    where
        X: PgExecutor<'e>,
    {
        let mut builder = QueryBuilder::<Postgres>::new("UPDATE ");
        builder
            .push(E::TABLE)
            .push(" SET is_active = FALSE, modified_by = ")
            .push_bind(modified_by)
            .push(" WHERE id = ")
            .push_bind(id);
        let result = builder.build().execute(executor).await?;
        Ok(result.rows_affected() > 0)
    }
}
