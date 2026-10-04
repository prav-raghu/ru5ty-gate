use admin_api::schemas::{
    BootstrapAdminRequest, BulkCreateUsersRequest, CustomBatchRequest, GenerateReportRequest,
    LoginRequest, OnboardingRequest, StreamReportQuery, Verify2FaRequest,
};
use serde_json::json;
use validator::Validate;

#[test]
fn login_requires_remember_me_and_rejects_unknown_fields() {
    assert!(
        serde_json::from_value::<LoginRequest>(
            json!({ "email": "a@b.com", "password": "longenough" })
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<LoginRequest>(
            json!({ "email": "a@b.com", "password": "longenough", "rememberMe": true, "x": 1 })
        )
        .is_err()
    );
    let ok: LoginRequest = serde_json::from_value(
        json!({ "email": "a@b.com", "password": "longenough", "rememberMe": true }),
    )
    .unwrap();
    assert!(ok.validate().is_ok());
}

#[test]
fn totp_codes_must_be_exactly_six_digits() {
    for (code, valid) in [
        ("123456", true),
        ("12345", false),
        ("1234567", false),
        ("12345a", false),
    ] {
        let request: Verify2FaRequest = serde_json::from_value(json!({ "token": code })).unwrap();
        assert_eq!(request.validate().is_ok(), valid, "{code}");
    }
}

#[test]
fn bootstrap_requests_follow_the_normal_password_policy() {
    let weak: BootstrapAdminRequest = serde_json::from_value(
        json!({ "username": "root", "email": "r@example.com", "password": "short" }),
    )
    .unwrap();
    let strong: BootstrapAdminRequest = serde_json::from_value(
        json!({ "username": "root", "email": "r@example.com", "password": "longenough" }),
    )
    .unwrap();

    assert!(weak.validate().is_err());
    assert!(strong.validate().is_ok());
}

#[test]
fn onboarding_requires_uuid_references() {
    let base = json!({
        "username": "New Hire", "email": "h@example.com", "password": "longenough",
        "allowEmailCommunications": true, "ipAddress": "1.1.1.1",
        "userStatusId": "not-a-uuid", "roleId": "also-not"
    });

    assert!(serde_json::from_value::<OnboardingRequest>(base).is_err());
}

#[test]
fn batch_requests_enforce_item_limits() {
    let none: BulkCreateUsersRequest = serde_json::from_value(json!({ "users": [] })).unwrap();
    let too_many: CustomBatchRequest = serde_json::from_value(json!({
        "operation": "CREATE",
        "items": (0..101).map(|index| json!({ "i": index })).collect::<Vec<_>>()
    }))
    .unwrap();
    let wrong_operation = serde_json::from_value::<CustomBatchRequest>(
        json!({ "operation": "CUSTOM", "items": [{}] }),
    );

    assert!(none.validate().is_err());
    assert!(too_many.validate().is_err());
    assert!(wrong_operation.is_err());
}

#[test]
fn report_requests_only_accept_the_documented_types_and_formats() {
    assert!(
        serde_json::from_value::<GenerateReportRequest>(
            json!({ "type": "USER_ACTIVITY", "format": "EXCEL" })
        )
        .is_ok()
    );
    assert!(
        serde_json::from_value::<GenerateReportRequest>(
            json!({ "type": "CUSTOM", "format": "CSV" })
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<GenerateReportRequest>(
            json!({ "type": "USER_ACTIVITY", "format": "DOCX" })
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<StreamReportQuery>(
            json!({ "type": "SYSTEM_METRICS", "format": "pdf" })
        )
        .is_err()
    );
}
