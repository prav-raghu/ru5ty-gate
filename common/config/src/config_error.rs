use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing required environment variable {key}")]
    Missing { key: String },
    #[error("invalid value for environment variable {key}: {reason}")]
    Invalid { key: String, reason: String },
}
