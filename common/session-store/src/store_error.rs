#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("failed to (de)serialize event payload: {0}")]
    Json(#[from] serde_json::Error),
    #[error("blocking task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("failed to create db directory {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unknown sync event kind: {0}")]
    UnknownEventKind(String),
    #[error("session store connection lock was poisoned")]
    Poisoned,
}

pub type Result<T> = std::result::Result<T, StoreError>;
