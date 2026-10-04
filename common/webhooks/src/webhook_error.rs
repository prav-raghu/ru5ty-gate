use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("database query failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("webhook payload could not be serialised: {0}")]
    Serialisation(#[from] serde_json::Error),
    #[error("webhook target must be a public http or https address")]
    TargetNotAllowed,
}
