use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("encryption key must be 64 hexadecimal characters")]
    InvalidKey,
    #[error("encrypted payload is malformed")]
    MalformedPayload,
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
}
