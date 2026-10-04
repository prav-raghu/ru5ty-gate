#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};
use ru5ty_gate_utilities::{
    ApiVersionManager, CryptoError, CryptoUtil, DateUtil, WebhookSignatureService,
};

const KEY: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

#[test]
fn crypto_round_trips_plaintext() {
    let crypto = CryptoUtil::from_hex_key(KEY).unwrap();

    let encrypted = crypto.encrypt("JBSWY3DPEHPK3PXP").unwrap();

    assert_ne!(encrypted, "JBSWY3DPEHPK3PXP");
    assert_eq!(crypto.decrypt(&encrypted).unwrap(), "JBSWY3DPEHPK3PXP");
}

#[test]
fn crypto_rejects_bad_key_and_tampering() {
    assert!(matches!(
        CryptoUtil::from_hex_key("abcd"),
        Err(CryptoError::InvalidKey)
    ));
    let crypto = CryptoUtil::from_hex_key(KEY).unwrap();
    let encrypted = crypto.encrypt("secret").unwrap();
    let tampered = format!("{encrypted}00");

    assert!(crypto.decrypt(&tampered).is_err());
    assert_eq!(
        crypto.decrypt("not-a-payload"),
        Err(CryptoError::MalformedPayload)
    );
}

#[test]
fn webhook_signature_verifies_only_matching_payload() {
    let service = WebhookSignatureService;
    let secret = service.generate_secret();
    let signature = service.generate_signature("{\"a\":1}", &secret);

    assert_eq!(secret.len(), 64);
    assert!(service.verify_signature("{\"a\":1}", &signature, &secret));
    assert!(!service.verify_signature("{\"a\":2}", &signature, &secret));
    assert!(!service.verify_signature("{\"a\":1}", "zz", &secret));
}

#[test]
fn date_util_parses_and_bounds_days() {
    let parsed = DateUtil::parse_iso_date("2026-03-04T10:20:30Z").unwrap();

    assert_eq!(DateUtil::to_iso_string(parsed), "2026-03-04T10:20:30.000Z");
    assert!(!DateUtil::is_valid_iso_date("04/03/2026"));
    assert_eq!(
        DateUtil::start_of_utc_day(parsed),
        Utc.with_ymd_and_hms(2026, 3, 4, 0, 0, 0).unwrap()
    );
    assert_eq!(
        DateUtil::add_days(parsed, 1).unwrap(),
        Utc.with_ymd_and_hms(2026, 3, 5, 10, 20, 30).unwrap()
    );
}

#[test]
fn api_versions_match_the_node_contract() {
    let manager = ApiVersionManager::default();

    assert_eq!(manager.current_version(), "v2");
    assert_eq!(manager.supported_versions(), vec!["v1", "v2"]);
    assert!(manager.version_info("v1").unwrap().is_deprecated);
    assert!(!manager.is_version_supported("v3"));
}

#[tokio::test]
async fn password_hashes_verify_and_unknown_users_never_match() {
    use ru5ty_gate_utilities::PasswordUtil;

    let hash = PasswordUtil::hash("correct horse").await.unwrap();

    assert!(PasswordUtil::verify("correct horse", &hash).await);
    assert!(!PasswordUtil::verify("wrong", &hash).await);
    assert!(PasswordUtil::verify_or_dummy("correct horse", Some(&hash)).await);
    assert!(!PasswordUtil::verify_or_dummy("correct horse", None).await);
}
