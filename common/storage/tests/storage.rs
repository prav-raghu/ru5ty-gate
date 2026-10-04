#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_config::EnvReader;
use ru5ty_gate_storage::{
    S3Config, StorageError, StorageService, build_object_key, validate_upload,
};
use ru5ty_gate_types::{StorageProvider, UploadOptions};

#[test]
fn object_keys_are_sanitised_and_foldered() {
    assert_eq!(
        build_object_key(Some("/avatars/"), "my file (1).png"),
        "avatars/my_file__1_.png"
    );
    assert_eq!(build_object_key(None, "a.txt"), "a.txt");
    assert_eq!(build_object_key(Some(""), "a.txt"), "a.txt");
}

#[test]
fn uploads_are_validated_for_size_and_mime() {
    let options = UploadOptions {
        max_size_bytes: Some(10),
        allowed_mime_types: Some(vec!["image/png".to_owned()]),
        ..UploadOptions::default()
    };

    assert_eq!(
        validate_upload(11, "image/png", &options),
        Err(StorageError::FileTooLarge(10))
    );
    assert_eq!(
        validate_upload(5, "text/plain", &options),
        Err(StorageError::MimeTypeNotAllowed("text/plain".to_owned()))
    );
    assert_eq!(validate_upload(5, "image/png", &options), Ok(()));
}

#[test]
fn r2_config_derives_endpoint_from_account() {
    let env = EnvReader::from_pairs([
        ("R2_BUCKET", "files"),
        ("R2_ACCESS_KEY_ID", "id"),
        ("R2_SECRET_ACCESS_KEY", "secret"),
        ("R2_ACCOUNT_ID", "abc123"),
    ]);

    let config = S3Config::from_env(&env, StorageProvider::R2).unwrap();

    assert_eq!(
        config.endpoint.as_deref(),
        Some("https://abc123.r2.cloudflarestorage.com")
    );
    assert_eq!(config.region, "auto");
}

#[test]
fn config_requires_credentials() {
    let env = EnvReader::from_pairs([("S3_BUCKET", "files")]);

    assert!(S3Config::from_env(&env, StorageProvider::S3).is_err());
}

#[test]
fn azure_blob_is_not_supported_yet() {
    let config = S3Config {
        provider: StorageProvider::AzureBlob,
        bucket: "b".to_owned(),
        region: "auto".to_owned(),
        endpoint: None,
        access_key_id: "i".to_owned(),
        secret_access_key: "s".to_owned(),
        public_base_url: None,
    };

    assert!(matches!(
        StorageService::new(config),
        Err(StorageError::UnsupportedProvider(_))
    ));
}
