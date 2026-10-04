#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use admin_api::schemas::{LoginRequest, ResetPasswordRequest, VerifyLoginMfaRequest};
use ru5ty_gate_database::sqlx::{self, PgPool};
use uuid::Uuid;

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, config_with, current_code,
    live_redis, unique_name,
};

fn login(email: &str, password: &str) -> LoginRequest {
    LoginRequest {
        email: email.to_owned(),
        password: password.to_owned(),
        remember_me: false,
    }
}

async fn app(
    pool: &PgPool,
    email: std::sync::Arc<RecordingEmailSender>,
) -> admin_api::application::Application {
    build_application(pool, email, live_redis().await, config_with(&[])).await
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn admin_tier_users_can_log_in_and_go_online(pool: PgPool) {
    let email = RecordingEmailSender::new();
    let app = app(&pool, email.clone()).await;
    let name = unique_name("Mod Person");
    let (id, address) = UserFactory::new(&pool, &name)
        .role("Moderator")
        .status("Offline")
        .allow_email()
        .create()
        .await;

    let result = app
        .state()
        .services
        .auth
        .login(&login(&address, TEST_PASSWORD), "10.1.1.1")
        .await
        .unwrap();

    assert!(result.is_successful);
    let data = result.data.unwrap();
    assert_eq!(data.username, name);
    assert!(!data.auth_token.is_empty() && !data.refresh_token.is_empty());
    assert!(data.mfa_required.is_none());
    let status: String = sqlx::query_scalar(
        "SELECT s.name FROM users u JOIN user_statuses s ON s.id = u.user_status_id WHERE u.id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "Online");
    let notification = email.last().unwrap();
    assert_eq!(notification.template, "admin-login-notification");
    assert_eq!(notification.variables["ipAddress"], "10.1.1.1");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn invalid_credentials_and_customer_accounts_get_the_same_generic_failure(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.auth;
    let (_, admin_email) = UserFactory::new(&pool, &unique_name("Real Admin"))
        .create()
        .await;
    let (_, customer_email) = UserFactory::new(&pool, &unique_name("Plain Customer"))
        .role("Chat User")
        .create()
        .await;
    let unknown = format!("{}@example.com", unique_name("ghost").replace(' ', ""));

    let wrong = service
        .login(&login(&admin_email, "WrongPassword1"), "ip")
        .await
        .unwrap();
    let customer = service
        .login(&login(&customer_email, TEST_PASSWORD), "ip")
        .await
        .unwrap();
    let missing = service
        .login(&login(&unknown, TEST_PASSWORD), "ip")
        .await
        .unwrap();

    for result in [wrong, customer, missing] {
        assert!(!result.is_successful);
        assert_eq!(
            result.message.as_deref(),
            Some("Invalid username or password")
        );
        let data = result.data.unwrap();
        assert!(data.auth_token.is_empty() && data.refresh_token.is_empty());
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn repeated_failures_lock_the_account_when_redis_is_available(pool: PgPool) {
    let redis = live_redis().await;
    if !redis.is_available() {
        return;
    }
    let app = build_application(&pool, RecordingEmailSender::new(), redis, config_with(&[])).await;
    let service = &app.state().services.auth;
    let (_, address) = UserFactory::new(&pool, &unique_name("Locked Admin"))
        .create()
        .await;

    for _ in 0..5 {
        service
            .login(&login(&address, "WrongPassword1"), "ip")
            .await
            .unwrap();
    }
    let locked = service
        .login(&login(&address, TEST_PASSWORD), "ip")
        .await
        .unwrap();

    assert!(!locked.is_successful);
    assert!(locked.message.unwrap().contains("temporarily locked"));
    app.state()
        .redis
        .del(&format!("login:fail:{address}"))
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn mfa_enabled_accounts_get_a_challenge_not_tokens_until_a_valid_code_is_supplied(
    pool: PgPool,
) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let name = unique_name("Mfa Admin");
    let (id, address) = UserFactory::new(&pool, &name).create().await;
    let users = &app.state().services.user;
    let setup = users.setup_2fa(id).await.unwrap().data.unwrap();
    let enable_code = current_code(&setup.secret);
    assert!(
        users
            .verify_2fa(id, &enable_code)
            .await
            .unwrap()
            .is_successful
    );

    let step_one = app
        .state()
        .services
        .auth
        .login(&login(&address, TEST_PASSWORD), "ip")
        .await
        .unwrap();
    let challenge = step_one.data.unwrap();
    assert!(step_one.is_successful);
    assert_eq!(challenge.mfa_required, Some(true));
    assert!(challenge.auth_token.is_empty());
    let mfa_token = challenge.mfa_token.unwrap();

    let wrong_code = if enable_code == "000000" {
        "111111"
    } else {
        "000000"
    };
    let rejected = app
        .state()
        .services
        .auth
        .verify_login_mfa(
            &VerifyLoginMfaRequest {
                mfa_token: mfa_token.clone(),
                code: wrong_code.to_owned(),
                remember_me: false,
            },
            "ip",
        )
        .await
        .unwrap();
    assert!(!rejected.is_successful);
    assert_eq!(
        rejected.message.as_deref(),
        Some("Invalid verification code")
    );

    let accepted = app
        .state()
        .services
        .auth
        .verify_login_mfa(
            &VerifyLoginMfaRequest {
                mfa_token,
                code: current_code(&setup.secret),
                remember_me: false,
            },
            "ip",
        )
        .await
        .unwrap();
    assert!(accepted.is_successful);
    assert!(!accepted.data.unwrap().auth_token.is_empty());

    let garbage = app
        .state()
        .services
        .auth
        .verify_login_mfa(
            &VerifyLoginMfaRequest {
                mfa_token: "not-a-token".to_owned(),
                code: "123456".to_owned(),
                remember_me: false,
            },
            "ip",
        )
        .await
        .unwrap();
    assert_eq!(
        garbage.message.as_deref(),
        Some("Verification session expired - please log in again")
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn forgot_and_reset_password_round_trip(pool: PgPool) {
    let email = RecordingEmailSender::new();
    let app = app(&pool, email.clone()).await;
    let service = &app.state().services.auth;
    let (_, address) = UserFactory::new(&pool, &unique_name("Forgetful Admin"))
        .create()
        .await;

    let neutral = service.forgot_password("nobody@example.com").await.unwrap();
    assert!(neutral.is_successful);
    assert_eq!(email.count(), 0);

    let sent = service.forgot_password(&address).await.unwrap();
    assert!(sent.is_successful);
    let mail = email.last().unwrap();
    assert_eq!(mail.template, "admin-forgot-password");
    let token = mail.variables["resetLink"]
        .rsplit("token=")
        .next()
        .unwrap()
        .to_owned();

    let mismatch = service
        .reset_password(&ResetPasswordRequest {
            token: Uuid::parse_str(&token).unwrap(),
            password: "BrandNewPass1".to_owned(),
            confirm_password: Some("Different1234".to_owned()),
        })
        .await
        .unwrap();
    assert_eq!(
        mismatch.message.as_deref(),
        Some("New password and confirm password do not match")
    );

    let reset = service
        .reset_password(&ResetPasswordRequest {
            token: Uuid::parse_str(&token).unwrap(),
            password: "BrandNewPass1".to_owned(),
            confirm_password: Some("BrandNewPass1".to_owned()),
        })
        .await
        .unwrap();
    assert!(reset.is_successful);
    let reuse = service
        .reset_password(&ResetPasswordRequest {
            token: Uuid::parse_str(&token).unwrap(),
            password: "BrandNewPass1".to_owned(),
            confirm_password: Some("BrandNewPass1".to_owned()),
        })
        .await
        .unwrap();
    assert_eq!(reuse.message.as_deref(), Some("Invalid or expired token"));
    assert!(
        service
            .login(&login(&address, "BrandNewPass1"), "ip")
            .await
            .unwrap()
            .is_successful
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn reset_rejects_expired_tokens_and_customer_accounts(pool: PgPool) {
    let email = RecordingEmailSender::new();
    let app = app(&pool, email.clone()).await;
    let service = &app.state().services.auth;
    let (_, address) = UserFactory::new(&pool, &unique_name("Slow Admin"))
        .create()
        .await;
    let (customer, _) = UserFactory::new(&pool, &unique_name("Customer Person"))
        .role("Chat User")
        .create()
        .await;
    service.forgot_password(&address).await.unwrap();
    let token = email.last().unwrap().variables["resetLink"]
        .rsplit("token=")
        .next()
        .unwrap()
        .to_owned();
    sqlx::query(
        "UPDATE users SET auth_hash_expiration = NOW() - INTERVAL '1 minute' WHERE email = $1",
    )
    .bind(&address)
    .execute(&pool)
    .await
    .unwrap();
    let customer_token = Uuid::new_v4();
    sqlx::query("UPDATE users SET auth_hash = $2, auth_hash_expiration = NOW() + INTERVAL '1 hour' WHERE id = $1")
        .bind(customer)
        .bind(customer_token.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let expired = service
        .reset_password(&ResetPasswordRequest {
            token: Uuid::parse_str(&token).unwrap(),
            password: "BrandNewPass1".to_owned(),
            confirm_password: Some("BrandNewPass1".to_owned()),
        })
        .await
        .unwrap();
    let foreign = service
        .reset_password(&ResetPasswordRequest {
            token: customer_token,
            password: "BrandNewPass1".to_owned(),
            confirm_password: Some("BrandNewPass1".to_owned()),
        })
        .await
        .unwrap();

    assert_eq!(expired.message.as_deref(), Some("Token has expired"));
    assert_eq!(foreign.message.as_deref(), Some("Invalid or expired token"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn logout_marks_the_user_offline_and_current_user_has_nested_role_and_status(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.auth;
    let (id, _) = UserFactory::new(&pool, &unique_name("Leaving Admin"))
        .create()
        .await;

    let current = service.get_current_user(id).await.unwrap().unwrap();
    assert_eq!(current.roles.name, "Super Admin");
    assert_eq!(current.status.name, "Online");

    let result = service.logout(id, None).await.unwrap();
    assert!(result.is_successful);
    assert_eq!(result.message.as_deref(), Some("Successfully Logged Out"));
    let after = service.get_current_user(id).await.unwrap().unwrap();
    assert_eq!(after.status.name, "Offline");
    assert!(
        service
            .get_current_user(Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}
