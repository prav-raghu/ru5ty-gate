use std::time::Duration;

use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use ru5ty_gate_types::{
    DeleteFileOptions, FileAccessLevel, FileMetadata, ListFilesOptions, ListFilesResult,
    SignedUrlOptions, SignedUrlResult, StorageProvider, UploadOptions, UploadResult,
};
use uuid::Uuid;

use crate::object_key::build_object_key;
use crate::s3_config::S3Config;
use crate::storage_error::StorageError;
use crate::upload_validation::validate_upload;

const DEFAULT_SIGNED_URL_SECONDS: u64 = 3600;

#[derive(Clone)]
pub struct StorageService {
    client: Client,
    config: S3Config,
}

fn operation_error(error: impl std::fmt::Display) -> StorageError {
    StorageError::Operation(error.to_string())
}

impl StorageService {
    pub fn new(config: S3Config) -> Result<Self, StorageError> {
        if !matches!(config.provider, StorageProvider::S3 | StorageProvider::R2) {
            return Err(StorageError::UnsupportedProvider(format!(
                "{:?}",
                config.provider
            )));
        }
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "environment",
        );
        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials);
        if let Some(endpoint) = &config.endpoint {
            builder = builder.endpoint_url(endpoint).force_path_style(true);
        }
        Ok(Self {
            client: Client::from_conf(builder.build()),
            config,
        })
    }

    fn bucket<'a>(&'a self, requested: Option<&'a str>) -> &'a str {
        requested.unwrap_or(&self.config.bucket)
    }

    fn public_url(&self, bucket: &str, key: &str) -> String {
        match &self.config.public_base_url {
            Some(base) => format!("{}/{key}", base.trim_end_matches('/')),
            None => format!(
                "{}/{bucket}/{key}",
                self.config
                    .endpoint
                    .as_deref()
                    .unwrap_or("https://s3.amazonaws.com")
                    .trim_end_matches('/')
            ),
        }
    }

    pub async fn upload_file(
        &self,
        bytes: Vec<u8>,
        original_name: &str,
        options: &UploadOptions,
    ) -> Result<UploadResult, StorageError> {
        let mime_type = options.content_type.clone().unwrap_or_else(|| {
            mime_guess::from_path(original_name)
                .first_or_octet_stream()
                .to_string()
        });
        let size_bytes = bytes.len() as u64;
        validate_upload(size_bytes, &mime_type, options)?;

        let id = Uuid::new_v4().to_string();
        let extension = std::path::Path::new(original_name)
            .extension()
            .and_then(|value| value.to_str());
        let file_name = options
            .file_name
            .clone()
            .unwrap_or_else(|| match extension {
                Some(extension) => format!("{id}.{extension}"),
                None => id.clone(),
            });
        let key = build_object_key(options.folder.as_deref(), &file_name);
        let bucket = self.bucket(options.bucket.as_deref()).to_owned();

        let mut request = self
            .client
            .put_object()
            .bucket(&bucket)
            .key(&key)
            .content_type(&mime_type)
            .body(ByteStream::from(bytes));
        if let Some(metadata) = &options.metadata {
            for (name, value) in metadata {
                request = request.metadata(name, value);
            }
        }
        request.send().await.map_err(operation_error)?;

        let is_public = options.access_level == Some(FileAccessLevel::Public);
        Ok(UploadResult {
            id,
            file_name,
            original_name: original_name.to_owned(),
            mime_type,
            size_bytes,
            url: self.public_url(&bucket, &key),
            provider: self.config.provider,
            bucket: Some(bucket),
            key: Some(key),
            uploaded_at: Utc::now(),
            is_public: Some(is_public),
            metadata: options.metadata.clone(),
        })
    }

    pub async fn delete_file(&self, options: &DeleteFileOptions) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(self.bucket(options.bucket.as_deref()))
            .key(&options.key)
            .send()
            .await
            .map_err(operation_error)?;
        Ok(())
    }

    pub async fn signed_url(
        &self,
        key: &str,
        bucket: Option<&str>,
        options: &SignedUrlOptions,
    ) -> Result<SignedUrlResult, StorageError> {
        let seconds = options.expires_in.unwrap_or(DEFAULT_SIGNED_URL_SECONDS);
        let presigning =
            PresigningConfig::expires_in(Duration::from_secs(seconds)).map_err(operation_error)?;
        let mut request = self
            .client
            .get_object()
            .bucket(self.bucket(bucket))
            .key(key);
        if let Some(content_type) = &options.response_content_type {
            request = request.response_content_type(content_type);
        }
        if let Some(disposition) = &options.response_content_disposition {
            request = request.response_content_disposition(disposition);
        }
        let presigned = request
            .presigned(presigning)
            .await
            .map_err(operation_error)?;
        Ok(SignedUrlResult {
            url: presigned.uri().to_owned(),
            expires_at: Utc::now() + chrono::Duration::seconds(i64::try_from(seconds).unwrap_or(0)),
        })
    }

    pub async fn list_files(
        &self,
        options: &ListFilesOptions,
    ) -> Result<ListFilesResult, StorageError> {
        let mut request = self
            .client
            .list_objects_v2()
            .bucket(self.bucket(options.bucket.as_deref()));
        if let Some(prefix) = &options.prefix {
            request = request.prefix(prefix);
        }
        if let Some(max_keys) = options.max_keys {
            request = request.max_keys(max_keys);
        }
        if let Some(token) = &options.continuation_token {
            request = request.continuation_token(token);
        }
        let response = request.send().await.map_err(operation_error)?;
        let files = response
            .contents()
            .iter()
            .map(|object| FileMetadata {
                file_name: object.key().unwrap_or_default().to_owned(),
                mime_type: mime_guess::from_path(object.key().unwrap_or_default())
                    .first_or_octet_stream()
                    .to_string(),
                size_bytes: u64::try_from(object.size().unwrap_or(0)).unwrap_or(0),
                uploaded_by: None,
                tags: None,
                custom_metadata: None,
            })
            .collect();
        Ok(ListFilesResult {
            files,
            continuation_token: response.next_continuation_token().map(str::to_owned),
            is_truncated: response.is_truncated().unwrap_or(false),
        })
    }
}
