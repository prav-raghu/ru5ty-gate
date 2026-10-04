use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("storage provider {0} is not supported")]
    UnsupportedProvider(String),
    #[error("file exceeds the maximum size of {0} bytes")]
    FileTooLarge(u64),
    #[error("mime type {0} is not allowed")]
    MimeTypeNotAllowed(String),
    #[error("no bucket configured")]
    MissingBucket,
    #[error("storage operation failed: {0}")]
    Operation(String),
}
