use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageProvider {
    S3,
    AzureBlob,
    R2,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileAccessLevel {
    Public,
    Private,
    Authenticated,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadOptions {
    pub bucket: Option<String>,
    pub folder: Option<String>,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub max_size_bytes: Option<u64>,
    pub allowed_mime_types: Option<Vec<String>>,
    pub access_level: Option<FileAccessLevel>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    pub id: String,
    pub file_name: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub url: String,
    pub provider: StorageProvider,
    pub bucket: Option<String>,
    pub key: Option<String>,
    pub uploaded_at: DateTime<Utc>,
    pub is_public: Option<bool>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedUrlOptions {
    pub expires_in: Option<u64>,
    pub response_content_type: Option<String>,
    pub response_content_disposition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedUrlResult {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub uploaded_by: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFileOptions {
    pub bucket: Option<String>,
    pub key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFilesOptions {
    pub bucket: Option<String>,
    pub prefix: Option<String>,
    pub max_keys: Option<i32>,
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFilesResult {
    pub files: Vec<FileMetadata>,
    pub continuation_token: Option<String>,
    pub is_truncated: bool,
}
