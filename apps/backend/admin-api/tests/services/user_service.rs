#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use admin_api::schemas::{OnboardingRequest, UpdateProfileRequest};
use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_http::AuthUser;
use ru5ty_gate_types::{Permission, RoleName, TokenScope, get_permissions_for_role};
use uuid::Uuid;

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, config_with, current_code,
    live_redis, unique_name,
};

async fn app(
    pool: &PgPool,
    email: std::sync::Arc<RecordingEmailSender>,
) -> admin_api::application::Application {
    build_application(pool, email, live_redis().await, config_with(&[])).await
}

fn actor(id: Uuid, role: RoleName) -> AuthUser {
    AuthUser {
        id,
        username: "actor".to_owned(),
        email: None,
        role: role.as_str().to_owned(),
        permissions: get_permissions_for_role(role),
        scope: TokenScope::Admin,
    }
}

async fn lookup(pool: &PgPool, table: &str, name: &str) -> Uuid {
    let query = match table {
        "roles" => "SELECT id FROM roles WHERE name = $1",
        _ => "SELECT id FROM user_statuses WHERE name = $1",
    };
    sqlx::query_scalar(query)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn onboarding(pool: &PgPool, username: &str, email: &str, role: &str) -> OnboardingRequest {
    OnboardingRequest {
        username: username.to_owned(),
        email: email.to_owned(),
        password: TEST_PASSWORD.to_owned(),
        gender: None,
        age: Some(33),
        country: None,
        region: None,
        allow_email_communications: true,
        accept_terms_and_conditions: Some(true),
        ip_address: "10.0.0.9".to_owned(),
        user_status_id: lookup(pool, "user_statuses", "Pending Verification").await,
        role_id: lookup(pool, "roles", role).await,
        join_date: None,
        avatar: None,
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn roles_and_statuses_are_listed_alphabetically(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;

    let roles = service.get_user_roles().await.unwrap();
    let statuses = service.get_user_statuses().await.unwrap();

    let role_names: Vec<_> = roles
        .data
        .unwrap()
        .into_iter()
        .map(|role| role.name)
        .collect();
    assert_eq!(
        role_names,
        vec!["Chat User", "Moderator", "Super Admin", "Support"]
    );
    assert_eq!(
        roles.message.as_deref(),
        Some("Roles retrieved successfully")
    );
    assert_eq!(statuses.data.unwrap().len(), 4);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn onboarding_creates_a_pending_user_attributed_to_the_actor_and_emails_them(pool: PgPool) {
    let email = RecordingEmailSender::new();
    let app = app(&pool, email.clone()).await;
    let service = &app.state().services.user;
    let (admin_id, _) = UserFactory::new(&pool, &unique_name("Onboarder"))
        .create()
        .await;

    let result = service
        .onboard_user(
            &onboarding(&pool, "New Hire", "hire@example.com", "Support").await,
            &actor(admin_id, RoleName::SuperAdmin),
        )
        .await
        .unwrap();

    assert!(result.is_successful);
    assert_eq!(
        result.message.as_deref(),
        Some("User onboarded successfully")
    );
    let (created_by, role): (String, String) = sqlx::query_as(
        "SELECT u.created_by, r.name FROM users u JOIN roles r ON r.id = u.role_id \
         WHERE u.email = 'hire@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created_by, admin_id.to_string());
    assert_eq!(role, "Support");
    let mail = email.last().unwrap();
    assert_eq!(mail.template, "admin-onboarding-notification");
    assert!(
        mail.variables["verificationLink"]
            .starts_with("http://localhost:4004/verify-email?auth_hash=")
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn onboarding_rejects_duplicates_disposable_emails_and_unknown_references(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let (admin_id, existing_email) = UserFactory::new(&pool, "Existing Person").create().await;
    let acting = actor(admin_id, RoleName::SuperAdmin);

    let dup_name = service
        .onboard_user(
            &onboarding(&pool, "Existing Person", "other@example.com", "Support").await,
            &acting,
        )
        .await
        .unwrap();
    let dup_email = service
        .onboard_user(
            &onboarding(&pool, "Someone New", &existing_email, "Support").await,
            &acting,
        )
        .await
        .unwrap();
    let disposable = service
        .onboard_user(
            &onboarding(&pool, "Burner Person", "x@mailinator.com", "Support").await,
            &acting,
        )
        .await
        .unwrap();
    let mut bad_role = onboarding(&pool, "Role Less", "roleless@example.com", "Support").await;
    bad_role.role_id = Uuid::new_v4();
    let missing_role = service.onboard_user(&bad_role, &acting).await.unwrap();
    let mut bad_status =
        onboarding(&pool, "Status Less", "statusless@example.com", "Support").await;
    bad_status.user_status_id = Uuid::new_v4();
    let missing_status = service.onboard_user(&bad_status, &acting).await.unwrap();

    assert_eq!(dup_name.message.as_deref(), Some("Username already exists"));
    assert_eq!(dup_email.message.as_deref(), Some("Email already exists"));
    assert_eq!(
        disposable.message.as_deref(),
        Some("Email address is not allowed")
    );
    assert_eq!(missing_role.message.as_deref(), Some("User role not found"));
    assert_eq!(
        missing_status.message.as_deref(),
        Some("User status not found")
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn only_actors_with_role_assign_can_create_super_admins(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let (admin_id, _) = UserFactory::new(&pool, &unique_name("Boss")).create().await;
    let request = onboarding(&pool, "Aspiring Admin", "aspire@example.com", "Super Admin").await;

    let moderator = service
        .onboard_user(&request, &actor(admin_id, RoleName::Moderator))
        .await;
    let super_admin = service
        .onboard_user(&request, &actor(admin_id, RoleName::SuperAdmin))
        .await;

    assert!(moderator.is_err());
    assert!(
        !actor(admin_id, RoleName::Moderator)
            .permissions
            .contains(&Permission::RoleAssign)
    );
    assert!(super_admin.unwrap().is_successful);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn resend_only_works_for_pending_users(pool: PgPool) {
    let email = RecordingEmailSender::new();
    let app = app(&pool, email.clone()).await;
    let service = &app.state().services.user;
    let (_, pending) = UserFactory::new(&pool, &unique_name("Pending Admin"))
        .status("Pending Verification")
        .create()
        .await;
    let (_, online) = UserFactory::new(&pool, &unique_name("Online Admin"))
        .create()
        .await;

    let missing = service
        .resend_verification_email("ghost@example.com")
        .await
        .unwrap();
    let wrong_state = service.resend_verification_email(&online).await.unwrap();
    let ok = service.resend_verification_email(&pending).await.unwrap();

    assert_eq!(missing.message.as_deref(), Some("User not found"));
    assert_eq!(
        wrong_state.message.as_deref(),
        Some("You cannot resend verification email for this user")
    );
    assert!(ok.is_successful);
    assert_eq!(email.count(), 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn availability_checks_reflect_existing_accounts(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let name = unique_name("Taken Admin");
    let (_, address) = UserFactory::new(&pool, &name).create().await;

    assert!(
        !service
            .is_email_available(&address)
            .await
            .unwrap()
            .available
    );
    assert!(
        service
            .is_email_available("free@example.com")
            .await
            .unwrap()
            .available
    );
    assert!(
        !service
            .is_username_available(&name)
            .await
            .unwrap()
            .available
    );
    assert!(
        service
            .is_username_available("Free Name")
            .await
            .unwrap()
            .available
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn profile_updates_enforce_uniqueness(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let (id, _) = UserFactory::new(&pool, &unique_name("Profile Owner"))
        .create()
        .await;
    let other_name = unique_name("Other Owner");
    let (_, other_email) = UserFactory::new(&pool, &other_name).create().await;

    let taken_name = service
        .update_profile(
            id,
            &UpdateProfileRequest {
                username: other_name,
                email: "mine@example.com".to_owned(),
                avatar: None,
            },
        )
        .await
        .unwrap();
    let taken_email = service
        .update_profile(
            id,
            &UpdateProfileRequest {
                username: "Fresh Name".to_owned(),
                email: other_email,
                avatar: None,
            },
        )
        .await
        .unwrap();
    let ok = service
        .update_profile(
            id,
            &UpdateProfileRequest {
                username: "Fresh Name".to_owned(),
                email: "mine@example.com".to_owned(),
                avatar: Some("https://img/x.png".to_owned()),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        taken_name.message.as_deref(),
        Some("Username already taken")
    );
    assert_eq!(taken_email.message.as_deref(), Some("Email already taken"));
    let data = ok.data.unwrap();
    assert_eq!(data.username, "Fresh Name");
    assert_eq!(data.avatar.as_deref(), Some("https://img/x.png"));
    let ghost = service
        .update_profile(
            Uuid::new_v4(),
            &UpdateProfileRequest {
                username: "Nobody".to_owned(),
                email: "n@example.com".to_owned(),
                avatar: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(ghost.message.as_deref(), Some("User not found"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn password_changes_require_the_current_password(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let (id, _) = UserFactory::new(&pool, &unique_name("Pw Admin"))
        .create()
        .await;

    let wrong = service
        .change_password(id, "WrongPassword1", "NewPassword12")
        .await
        .unwrap();
    let ok = service
        .change_password(id, TEST_PASSWORD, "NewPassword12")
        .await
        .unwrap();
    let old_now_wrong = service
        .change_password(id, TEST_PASSWORD, "Another12345")
        .await
        .unwrap();

    assert_eq!(
        wrong.message.as_deref(),
        Some("Current password is incorrect")
    );
    assert!(ok.is_successful);
    assert!(!old_now_wrong.is_successful);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn two_factor_lifecycle_requires_real_totp_codes_and_encrypts_the_secret(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let (id, _) = UserFactory::new(&pool, &unique_name("Totp Admin"))
        .create()
        .await;

    let early = service.verify_2fa(id, "123456").await.unwrap();
    assert_eq!(
        early.message.as_deref(),
        Some("User not found or 2FA not set up")
    );

    let setup = service.setup_2fa(id).await.unwrap().data.unwrap();
    assert!(setup.qr_code.starts_with("data:image/png;base64,"));
    let stored: String = sqlx::query_scalar("SELECT two_factor_secret FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!stored.contains(&setup.secret));
    assert_eq!(stored.split(':').count(), 3);

    let valid = current_code(&setup.secret);
    let wrong = if valid == "000000" {
        "111111"
    } else {
        "000000"
    };
    assert_eq!(
        service
            .verify_2fa(id, wrong)
            .await
            .unwrap()
            .message
            .as_deref(),
        Some("Invalid verification code")
    );
    assert!(service.verify_2fa(id, &valid).await.unwrap().is_successful);
    let enabled: bool = sqlx::query_scalar("SELECT two_factor_enabled FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(enabled);

    assert!(!service.disable_2fa(id, wrong).await.unwrap().is_successful);
    assert!(
        service
            .disable_2fa(id, &current_code(&setup.secret))
            .await
            .unwrap()
            .is_successful
    );
    let (enabled, secret): (bool, Option<String>) =
        sqlx::query_as("SELECT two_factor_enabled, two_factor_secret FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!enabled);
    assert!(secret.is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn user_details_include_nested_status_and_role(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let name = unique_name("Detail Admin");
    let (id, _) = UserFactory::new(&pool, &name).create().await;

    let details = service.get_user_details(id).await.unwrap().unwrap();

    assert!(details.is_successful);
    assert_eq!(details.data.username, name);
    assert_eq!(details.data.status.name, "Online");
    assert_eq!(details.data.roles.name, "Super Admin");
    assert_eq!(details.data.ip_addresses, vec!["127.0.0.1".to_owned()]);
    assert!(
        service
            .get_user_details(Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn authorised_admin_lookup_requires_online_admin_tier_active_accounts(pool: PgPool) {
    let app = app(&pool, RecordingEmailSender::new()).await;
    let service = &app.state().services.user;
    let (online, _) = UserFactory::new(&pool, &unique_name("Ok Admin"))
        .create()
        .await;
    let (offline, _) = UserFactory::new(&pool, &unique_name("Off Admin"))
        .status("Offline")
        .create()
        .await;
    let (customer, _) = UserFactory::new(&pool, &unique_name("Cust"))
        .role("Chat User")
        .create()
        .await;
    let (inactive, _) = UserFactory::new(&pool, &unique_name("Dead Admin"))
        .create()
        .await;
    sqlx::query("UPDATE users SET is_active = FALSE WHERE id = $1")
        .bind(inactive)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        service
            .get_authorized_admin(online)
            .await
            .unwrap()
            .is_some()
    );
    for id in [offline, customer, inactive, Uuid::new_v4()] {
        assert!(service.get_authorized_admin(id).await.unwrap().is_none());
    }
}
