#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_config::EnvReader;
use ru5ty_gate_sms::{SmsConfig, SmsPortalResponse, SmsSender, SmsService, to_e164};

#[test]
fn converts_south_african_numbers_to_e164() {
    assert_eq!(to_e164("082 123 4567").as_deref(), Some("27821234567"));
    assert_eq!(to_e164("+27 82 123 4567").as_deref(), Some("27821234567"));
    assert_eq!(to_e164("27821234567").as_deref(), Some("27821234567"));
}

#[test]
fn rejects_numbers_it_cannot_normalise() {
    assert_eq!(to_e164("12345"), None);
    assert_eq!(to_e164("+1 415 555 2671"), None);
}

#[test]
fn portal_response_requires_accepted_messages_and_no_faults() {
    let accepted: SmsPortalResponse =
        serde_json::from_str(r#"{"sendResponse":{"messages":1,"errorReport":{"faults":[]}}}"#)
            .unwrap();
    let faulted: SmsPortalResponse = serde_json::from_str(
        r#"{"sendResponse":{"messages":1,"errorReport":{"faults":[{"faultId":"x"}]}}}"#,
    )
    .unwrap();
    let errored: SmsPortalResponse = serde_json::from_str(r#"{"errors":["bad"]}"#).unwrap();

    assert!(accepted.is_accepted());
    assert!(!faulted.is_accepted());
    assert!(!errored.is_accepted());
}

#[test]
fn config_defaults_to_enabled_and_respects_false() {
    assert!(SmsConfig::from_env(&EnvReader::default()).enabled);
    assert!(!SmsConfig::from_env(&EnvReader::from_pairs([("SMSPORTAL_ENABLED", "false")])).enabled);
}

#[tokio::test]
async fn send_is_skipped_when_disabled_or_unconfigured() {
    let disabled = SmsService::new(SmsConfig::from_env(&EnvReader::from_pairs([(
        "SMSPORTAL_ENABLED",
        "false",
    )])));
    let unconfigured = SmsService::new(SmsConfig::from_env(&EnvReader::default()));

    assert!(!disabled.send_sms("0821234567", "hi").await);
    assert!(!unconfigured.send_sms("0821234567", "hi").await);
}
