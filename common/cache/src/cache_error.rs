use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("redis is unavailable")]
    Unavailable,
    #[error("redis operation failed: {0}")]
    Operation(#[from] redis::RedisError),
    #[error("cached value could not be serialised: {0}")]
    Serialisation(#[from] serde_json::Error),
}
