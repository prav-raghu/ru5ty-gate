use sqlx::PgExecutor;

use crate::database_error::DatabaseError;
use crate::models::Venue;

pub struct VenueRepository;

impl VenueRepository {
    pub async fn find_active_by_code<'e, E>(
        executor: E,
        code: &str,
    ) -> Result<Option<Venue>, DatabaseError>
    where
        E: PgExecutor<'e>,
    {
        let venue =
            sqlx::query_as::<_, Venue>("SELECT * FROM venues WHERE code = $1 AND is_active = TRUE")
                .bind(code)
                .fetch_optional(executor)
                .await?;
        Ok(venue)
    }
}
