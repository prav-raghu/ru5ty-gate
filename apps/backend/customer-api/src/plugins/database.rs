use ru5ty_gate_database::{DatabaseError, PgPool, connect};

use crate::config::ServiceConfig;

pub async fn connect_database(config: &ServiceConfig) -> Result<PgPool, DatabaseError> {
    connect(&config.database).await
}
