use ru5ty_gate_types::UploadOptions;

use crate::storage_error::StorageError;

pub fn validate_upload(
    size_bytes: u64,
    mime_type: &str,
    options: &UploadOptions,
) -> Result<(), StorageError> {
    if let Some(max) = options.max_size_bytes
        && size_bytes > max
    {
        return Err(StorageError::FileTooLarge(max));
    }
    if let Some(allowed) = &options.allowed_mime_types
        && !allowed.iter().any(|candidate| candidate == mime_type)
    {
        return Err(StorageError::MimeTypeNotAllowed(mime_type.to_owned()));
    }
    Ok(())
}
