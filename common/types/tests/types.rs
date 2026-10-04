#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_types::{
    ApiResponse, FieldError, Permission, RoleName, get_permissions_for_role, is_disposable_email,
    role_has_permission,
};

#[test]
fn super_admin_has_every_permission() {
    let permissions = get_permissions_for_role(RoleName::SuperAdmin);

    assert_eq!(permissions.len(), Permission::ALL.len());
}

#[test]
fn support_cannot_write_batches() {
    assert!(!role_has_permission(
        RoleName::Support,
        Permission::BatchWrite
    ));
    assert!(role_has_permission(
        RoleName::Moderator,
        Permission::BatchWrite
    ));
}

#[test]
fn role_names_round_trip() {
    assert_eq!(
        RoleName::from_name("Super Admin"),
        Some(RoleName::SuperAdmin)
    );
    assert_eq!(RoleName::SuperAdmin.as_str(), "Super Admin");
    assert!(RoleName::Moderator.is_admin_tier());
    assert!(RoleName::ChatUser.is_customer_tier());
}

#[test]
fn permission_serialises_to_wire_value() {
    let json = serde_json::to_string(&Permission::ReportExport).unwrap();

    assert_eq!(json, "\"report:export\"");
}

#[test]
fn disposable_domains_are_rejected_case_insensitively() {
    assert!(is_disposable_email("someone@Mailinator.com"));
    assert!(!is_disposable_email("someone@example.com"));
    assert!(!is_disposable_email("not-an-email"));
}

#[test]
fn envelope_serialises_with_camel_case_and_skips_empty_fields() {
    let body = serde_json::to_value(ApiResponse::<()>::validation_failure(vec![FieldError {
        field: "email".to_owned(),
        message: "Must be a valid email address".to_owned(),
    }]))
    .unwrap();

    assert_eq!(body["isSuccessful"], false);
    assert_eq!(body["message"], "Validation failed");
    assert_eq!(body["errors"][0]["field"], "email");
    assert!(body.get("data").is_none());
}
