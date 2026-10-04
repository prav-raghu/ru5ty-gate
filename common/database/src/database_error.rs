use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Config(#[from] ru5ty_gate_config::ConfigError),
    #[error("database query failed: {0}")]
    Query(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("password hashing failed: {0}")]
    Hashing(#[from] bcrypt::BcryptError),
    #[error("seed prerequisite missing: {0}")]
    SeedPrerequisite(String),
}

impl DatabaseError {
    pub fn is_unique_violation(&self) -> bool {
        matches!(
            self,
            Self::Query(error)
                if error
                    .as_database_error()
                    .is_some_and(|database| database.is_unique_violation())
        )
    }
}
