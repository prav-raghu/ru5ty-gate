use sqlx::PgExecutor;
use uuid::Uuid;

use crate::database_error::DatabaseError;
use crate::inputs::UserListFilter;
use crate::records::{AuthorizedUserRecord, ExportUserRecord, UserSummaryRecord};

pub struct UserRepository;

impl UserRepository {
    pub async fn list_active<'e, E>(
        executor: E,
        filter: &UserListFilter<'_>,
    ) -> Result<Vec<UserSummaryRecord>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let records = sqlx::query_as::<_, UserSummaryRecord>(
            "SELECT id, username, age, last_seen FROM users \
             WHERE is_active = TRUE \
               AND ($1::uuid IS NULL OR id <> $1) \
               AND ($2::text IS NULL OR gender = $2) \
               AND ($3::int IS NULL OR age >= $3) \
               AND ($4::int IS NULL OR age <= $4) \
             ORDER BY last_seen DESC, id \
             LIMIT $5 OFFSET $6",
        )
        .bind(filter.exclude_user_id)
        .bind(filter.gender)
        .bind(filter.min_age)
        .bind(filter.max_age)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(executor)
        .await?;
        Ok(records)
    }

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

    pub async fn find_active_with_role<'e, E>(
        executor: E,
        user_id: Uuid,
        role_name: &str,
    ) -> Result<Option<AuthorizedUserRecord>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let record = sqlx::query_as::<_, AuthorizedUserRecord>(
            "SELECT u.id, u.username, u.email, r.name AS role_name \
             FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE u.id = $1 AND u.is_active = TRUE AND r.name = $2",
        )
        .bind(user_id)
        .bind(role_name)
        .fetch_optional(executor)
        .await?;
        Ok(record)
    }

    pub async fn list_for_export<'e, E>(
        executor: E,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ExportUserRecord>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let records = sqlx::query_as::<_, ExportUserRecord>(
            "SELECT id, email, username, created_at FROM users \
             ORDER BY created_at DESC, id LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;
        Ok(records)
    }
}
