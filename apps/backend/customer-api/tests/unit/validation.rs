use customer_api::schemas::{
    CreateWebhookSubscriptionRequest, ExportFormat, ExportQuery, RegisterRequest,
};
use customer_api::services::auth_service::{is_email_domain_allowed, is_username_valid};
use serde_json::json;
use validator::Validate;

#[test]
fn usernames_allow_letters_underscores_and_spaces_only() {
    assert!(is_username_valid("Pat Smith"));
    assert!(is_username_valid("pat_smith"));
    assert!(!is_username_valid("pat1"));
    assert!(!is_username_valid("   "));
    assert!(!is_username_valid("100%"));
    assert!(!is_username_valid("pat\\smith"));
    assert!(!is_username_valid("pat\u{0007}"));
}

#[test]
fn email_domains_reject_disposable_providers_case_insensitively() {
    assert!(is_email_domain_allowed("someone@example.com"));
    assert!(!is_email_domain_allowed("someone@Mailinator.com"));
    assert!(!is_email_domain_allowed("someone@yopmail.com"));
    assert!(!is_email_domain_allowed("no-at-sign"));
}

#[test]
fn register_requests_enforce_bounds_and_reject_unknown_fields() {
    let valid = json!({
        "username": "Pat Smith", "password": "longenough", "email": "pat@example.com",
        "age": 30, "genderId": "x", "acceptTermsAndConditions": true
    });
    let mut too_young = valid.clone();
    too_young["age"] = json!(12);
    let mut extra = valid.clone();
    extra["role"] = json!("admin");

    assert!(
        serde_json::from_value::<RegisterRequest>(valid)
            .unwrap()
            .validate()
            .is_ok()
    );
    assert!(
        serde_json::from_value::<RegisterRequest>(too_young)
            .unwrap()
            .validate()
            .is_err()
    );
    assert!(serde_json::from_value::<RegisterRequest>(extra).is_err());
}

#[test]
fn webhook_subscriptions_require_known_events_and_valid_urls() {
    let valid: CreateWebhookSubscriptionRequest = serde_json::from_value(
        json!({ "url": "https://example.com/h", "events": ["user.created"] }),
    )
    .unwrap();
    let unknown: CreateWebhookSubscriptionRequest =
        serde_json::from_value(json!({ "url": "https://example.com/h", "events": ["made.up"] }))
            .unwrap();
    let bad_url: CreateWebhookSubscriptionRequest =
        serde_json::from_value(json!({ "url": "nope", "events": ["user.created"] })).unwrap();
    let short_secret: CreateWebhookSubscriptionRequest = serde_json::from_value(
        json!({ "url": "https://example.com/h", "events": ["user.created"], "secret": "short" }),
    )
    .unwrap();

    assert!(valid.validate().is_ok());
    assert!(unknown.validate().is_err());
    assert!(bad_url.validate().is_err());
    assert!(short_secret.validate().is_err());
}

#[test]
fn export_format_defaults_to_csv_and_maps_content_types() {
    let default: ExportQuery = serde_json::from_value(json!({})).unwrap();
    let excel: ExportQuery = serde_json::from_value(json!({ "format": "excel" })).unwrap();

    assert_eq!(default.format, ExportFormat::Csv);
    assert_eq!(excel.format.extension(), ".xlsx");
    assert!(excel.format.content_type().contains("spreadsheetml"));
    assert!(serde_json::from_value::<ExportQuery>(json!({ "format": "pdf" })).is_err());
}
