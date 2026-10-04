use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::database_config::DatabaseConfig;
use crate::database_error::DatabaseError;

pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, DatabaseError> {
    Ok(PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(&config.url)
        .await?)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
