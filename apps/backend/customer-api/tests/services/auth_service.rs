#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use customer_api::schemas::{LoginRequest, RegisterRequest};
use ru5ty_gate_database::sqlx::{self, PgPool};

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, live_redis, unique_name,
};

fn register_request(username: &str, email: &str) -> RegisterRequest {
    RegisterRequest {
        username: username.to_owned(),
        password: TEST_PASSWORD.to_owned(),
        email: email.to_owned(),
        age: 25,
        gender_id: "female".to_owned(),
        accept_terms_and_conditions: true,
        allow_email_communications: false,
    }
}

fn login_request(username: &str, password: &str) -> LoginRequest {
    LoginRequest {
        username: username.to_owned(),
        password: password.to_owned(),
        remember_me: Some(false),
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn register_creates_a_pending_user_and_sends_a_verification_email(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email.clone(), live_redis().await).await;
    let service = &app.state().services.auth;

    let result = service
        .register(
            &register_request("Pat Smith", "pat@example.com"),
            "10.0.0.1",
        )
        .await
        .unwrap();

    assert!(result.is_successful);
    assert_eq!(result.data.unwrap().email, "pat@example.com");
    let sent = email.last().unwrap();
    assert_eq!(sent.template, "verify-email");
    assert!(
        sent.variables["verificationLink"]
            .starts_with("http://localhost:3000/verify-email?email=pat%40example.com&token=")
    );
    let status: String = sqlx::query_scalar(
        "SELECT s.name FROM users u JOIN user_statuses s ON s.id = u.user_status_id \
         WHERE u.email = 'pat@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "Pending Verification");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn register_rejects_invalid_requests_with_friendly_messages(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email.clone(), live_redis().await).await;
    let service = &app.state().services.auth;
    UserFactory::new(&pool, "Taken Name").create().await;

    let mut no_terms = register_request("Alice Doe", "alice@example.com");
    no_terms.accept_terms_and_conditions = false;
    let cases = [
        (
            no_terms,
            "You must accept the terms and conditions to register",
        ),
        (
            register_request("Bad1Name", "a1@example.com"),
            "Username contains invalid characters",
        ),
        (
            register_request("Taken Name", "b@example.com"),
            "Username is already taken",
        ),
        (
            register_request("Fresh Name", "c@mailinator.com"),
            "Email address is not allowed",
        ),
        (
            register_request("Other Name", "takenname@example.com"),
            "Email is already registered",
        ),
    ];

    for (request, expected) in cases {
        let result = service.register(&request, "10.0.0.1").await.unwrap();
        assert!(!result.is_successful, "{expected}");
        assert_eq!(result.message.as_deref(), Some(expected));
    }
    assert_eq!(email.count(), 0);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn register_reports_email_delivery_failures(pool: PgPool) {
    let email = RecordingEmailSender::new(false);
    let app = build_application(&pool, email, live_redis().await).await;

    let result = app
        .state()
        .services
        .auth
        .register(
            &register_request("Mail Fail", "fail@example.com"),
            "10.0.0.1",
        )
        .await
        .unwrap();

    assert!(!result.is_successful);
    assert_eq!(
        result.message.as_deref(),
        Some("Failed to send user verification email")
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn login_requires_verification_and_the_customer_tier(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email, live_redis().await).await;
    let service = &app.state().services.auth;
    let verified_name = unique_name("Verified User");
    let admin_name = unique_name("Admin Person");
    let unknown_name = unique_name("Nobody Here");
    UserFactory::new(&pool, "Pending User")
        .status("Pending Verification")
        .create()
        .await;
    UserFactory::new(&pool, &verified_name).create().await;
    UserFactory::new(&pool, &admin_name)
        .role("Super Admin")
        .create()
        .await;

    let pending = service
        .login(&login_request("Pending User", TEST_PASSWORD))
        .await
        .unwrap();
    let verified = service
        .login(&login_request(&verified_name, TEST_PASSWORD))
        .await
        .unwrap();
    let wrong = service
        .login(&login_request(&verified_name, "WrongPassword1"))
        .await
        .unwrap();
    let admin = service
        .login(&login_request(&admin_name, TEST_PASSWORD))
        .await
        .unwrap();
    let unknown = service
        .login(&login_request(&unknown_name, TEST_PASSWORD))
        .await
        .unwrap();

    assert_eq!(
        pending.message.as_deref(),
        Some("Please verify your email before logging in")
    );
    assert!(verified.is_successful);
    let data = verified.data.unwrap();
    assert_eq!(data.user_name, verified_name);
    assert!(!data.access_token.is_empty());
    for failed in [wrong, admin, unknown] {
        assert!(!failed.is_successful);
        assert_eq!(
            failed.message.as_deref(),
            Some("Invalid username or password")
        );
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn login_marks_the_user_online(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    UserFactory::new(&pool, "Online User").create().await;

    app.state()
        .services
        .auth
        .login(&login_request("Online User", TEST_PASSWORD))
        .await
        .unwrap();

    let status: String = sqlx::query_scalar(
        "SELECT s.name FROM users u JOIN user_statuses s ON s.id = u.user_status_id \
         WHERE u.username = 'Online User'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "Online");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn verify_email_accepts_a_fresh_token_once(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email.clone(), live_redis().await).await;
    let service = &app.state().services.auth;
    service
        .register(
            &register_request("Verify Me", "verify@example.com"),
            "10.0.0.1",
        )
        .await
        .unwrap();
    let link = email.last().unwrap().variables["verificationLink"].clone();
    let token = link.rsplit("token=").next().unwrap().to_owned();

    let first = service.verify_email(&token).await.unwrap();
    let second = service.verify_email(&token).await.unwrap();
    let invalid = service.verify_email("not-a-token").await.unwrap();

    assert!(first.is_successful);
    assert_eq!(
        first.message.as_deref(),
        Some("Email verified successfully")
    );
    assert!(!second.is_successful);
    assert_eq!(invalid.message.as_deref(), Some("Invalid or expired token"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn verify_email_rejects_expired_tokens(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email.clone(), live_redis().await).await;
    let service = &app.state().services.auth;
    service
        .register(
            &register_request("Late User", "late@example.com"),
            "10.0.0.1",
        )
        .await
        .unwrap();
    sqlx::query("UPDATE users SET auth_hash_expiration = NOW() - INTERVAL '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let token = email.last().unwrap().variables["verificationLink"]
        .rsplit("token=")
        .next()
        .unwrap()
        .to_owned();

    let result = service.verify_email(&token).await.unwrap();

    assert!(!result.is_successful);
    assert_eq!(result.message.as_deref(), Some("Invalid or expired token"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn resend_is_neutral_and_only_mails_unverified_users(pool: PgPool) {
    let email = RecordingEmailSender::new(true);
    let app = build_application(&pool, email.clone(), live_redis().await).await;
    let service = &app.state().services.auth;
    UserFactory::new(&pool, "Waiting User")
        .status("Pending Verification")
        .create()
        .await;
    UserFactory::new(&pool, "Done User").create().await;

    let unknown = service
        .resend_verification_email("nobody@example.com")
        .await
        .unwrap();
    let verified = service
        .resend_verification_email("doneuser@example.com")
        .await
        .unwrap();
    assert_eq!(email.count(), 0);
    let pending = service
        .resend_verification_email("waitinguser@example.com")
        .await
        .unwrap();

    assert_eq!(email.count(), 1);
    for result in [unknown, verified, pending] {
        assert!(result.is_successful);
        assert_eq!(
            result.message.as_deref(),
            Some("If that email is registered and unverified, a new link has been sent")
        );
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn repeated_failures_lock_the_account_when_redis_is_available(pool: PgPool) {
    let redis = live_redis().await;
    if !redis.is_available() {
        return;
    }
    let app = build_application(&pool, RecordingEmailSender::new(true), redis).await;
    let service = &app.state().services.auth;
    let name = unique_name("Lock Target");
    UserFactory::new(&pool, &name).create().await;

    for _ in 0..5 {
        service
            .login(&login_request(&name, "WrongPassword1"))
            .await
            .unwrap();
    }
    let locked = service
        .login(&login_request(&name, TEST_PASSWORD))
        .await
        .unwrap();

    assert!(!locked.is_successful);
    assert!(locked.message.unwrap().contains("temporarily locked"));
    app.state()
        .redis
        .del("login:fail:Lock Target")
        .await
        .unwrap();
}
