# File Upload System Documentation

## Overview

File storage is provided by the `common/storage` crate (`ru5ty-gate-storage`), an S3-compatible client built on the AWS SDK. It works with AWS S3 and Cloudflare R2. Azure Blob storage is not supported by the Rust crate; use an S3-compatible gateway or add a provider behind the same `StorageService` surface if it is needed.

No service wires file upload endpoints by default. This document describes the crate and the pattern for adding an upload endpoint to a service.

## Configuration

`S3Config::from_env(&env, provider)` reads one prefix per provider.

| Variable (S3) | Variable (R2) | Notes |
|---|---|---|
| `S3_BUCKET` | `R2_BUCKET` | Required |
| `S3_ACCESS_KEY_ID` | `R2_ACCESS_KEY_ID` | Required, secret |
| `S3_SECRET_ACCESS_KEY` | `R2_SECRET_ACCESS_KEY` | Required, secret |
| `S3_REGION` | `R2_REGION` | Defaults to `auto` |
| `S3_ENDPOINT` | `R2_ENDPOINT` or `R2_ACCOUNT_ID` | R2 builds `https://<account>.r2.cloudflarestorage.com` from the account id |
| `S3_PUBLIC_URL` | `R2_PUBLIC_URL` | Optional base URL for public files |

```rust
let config = S3Config::from_env(&env, StorageProvider::R2)?;
let storage = StorageService::new(config)?;
```

Build the `StorageService` once in `plugins/services.rs` and keep it in the service container.

## Uploading

```rust
let options = UploadOptions {
    folder: Some("profile-images".to_owned()),
    max_size_bytes: Some(5 * 1024 * 1024),
    allowed_mime_types: Some(vec!["image/jpeg".to_owned(), "image/png".to_owned()]),
    access_level: Some(FileAccessLevel::Private),
    ..UploadOptions::default()
};
let result = storage.upload_file(bytes, "avatar.png", &options).await?;
```

`upload_file` validates size and MIME type (`validate_upload`) before touching the network, generates a UUID file name that keeps the extension, builds the object key from the folder (`build_object_key`), uploads the bytes and returns an `UploadResult` (`id`, `fileName`, `url`, `key`, `bucket`, `sizeBytes`, ...). The MIME type comes from `content_type` when given, otherwise it is guessed from the file name, so for security-sensitive uploads also check the file's magic bytes before calling it.

Other operations: `delete_file(&DeleteFileOptions)`, `list_files(&ListFilesOptions)`, and `signed_url(key, bucket, &SignedUrlOptions)` for temporary access (default expiry applies when `expires_in` is `None`).

## Access levels

| Level | Use |
|---|---|
| `Public` | Served through `S3_PUBLIC_URL`; the returned URL is permanent |
| `Private` | Never exposed directly; hand out signed URLs |
| `Authenticated` | Private object, signed URL generated only after your own permission check |

## Adding an upload endpoint (Axum)

Enable the Axum `multipart` feature in the service's `Cargo.toml`, then:

```rust
pub async fn upload_profile_image(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart body".to_owned()))?
        .ok_or_else(|| AppError::BadRequest("No file provided".to_owned()))?;
    let name = field.file_name().unwrap_or("upload").to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| AppError::BadRequest("Could not read file".to_owned()))?;
    let result = state.services.storage.upload_file(bytes.to_vec(), &name, &profile_image_options()).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(result))).into_response())
}
```

Map `StorageError::FileTooLarge` and `StorageError::MimeTypeNotAllowed` to `AppError::BadRequest`, and everything else to `AppError::internal`. Set an Axum `DefaultBodyLimit` on the route so oversized bodies are rejected before they are buffered.

## Security best practices

1. Validate type (extension, declared MIME type and magic bytes) and size before storing.
2. Apply the `sensitive_endpoints` rate-limit policy to upload routes.
3. Authenticate and authorise every upload and every signed-URL request; scope object keys by owner id.
4. Prefer signed URLs over public buckets for user content.
5. Scan untrusted files for malware where the product requires it.
6. Never log file contents or full signed URLs.

## Testing

Unit-test `validate_upload` and `build_object_key` directly (`common/storage/tests`). Test endpoints against a local S3-compatible server (for example MinIO) started in the test, or put `StorageService` behind a trait in the service and use a recording fake, the same way `EmailSender` is faked.
