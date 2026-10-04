use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("redis operation failed: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("job could not be serialised: {0}")]
    Serialisation(#[from] serde_json::Error),
}
