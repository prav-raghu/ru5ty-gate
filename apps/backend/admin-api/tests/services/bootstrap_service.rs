#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use admin_api::schemas::BootstrapAdminRequest;
use ru5ty_gate_database::sqlx::{self, PgPool};

use crate::common::{
    RecordingEmailSender, TEST_PASSWORD, UserFactory, build_application, config_with, live_redis,
};

fn request(username: &str, email: &str) -> BootstrapAdminRequest {
    BootstrapAdminRequest {
        username: username.to_owned(),
        email: email.to_owned(),
        password: TEST_PASSWORD.to_owned(),
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn creates_the_first_super_admin_once_and_locks_permanently(pool: PgPool) {
    let app = build_application(
        &pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await;
    let service = &app.state().services.bootstrap;

    let first = service
        .bootstrap_admin(&request("first admin", "first@example.com"))
        .await
        .unwrap();
    let second = service
        .bootstrap_admin(&request("second admin", "second@example.com"))
        .await
        .unwrap();
    sqlx::query("DELETE FROM users")
        .execute(&pool)
        .await
        .unwrap();
    let after_delete = service
        .bootstrap_admin(&request("third admin", "third@example.com"))
        .await
        .unwrap();

    assert!(first.is_successful);
    assert!(!second.is_successful);
    assert_eq!(
        second.message.as_deref(),
        Some("An administrator account already exists")
    );
    assert!(!after_delete.is_successful);
    let (flag, user_id): (bool, Option<uuid::Uuid>) = sqlx::query_as(
        "SELECT admin_bootstrapped, bootstrapped_user_id FROM system_bootstrap WHERE id = 'singleton'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(flag);
    assert!(user_id.is_some());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn the_created_admin_has_the_super_admin_role_and_a_strong_hash(pool: PgPool) {
    let app = build_application(
        &pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await;

    app.state()
        .services
        .bootstrap
        .bootstrap_admin(&request("root admin", "root@example.com"))
        .await
        .unwrap();

    let (role, hash): (String, String) =
        sqlx::query_as("SELECT r.name, u.password FROM users u JOIN roles r ON r.id = u.role_id")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(role, "Super Admin");
    assert!(
        hash.starts_with("$2b$12$") || hash.starts_with("$2a$12$") || hash.starts_with("$2y$12$")
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn is_refused_when_the_env_kill_switch_is_off(pool: PgPool) {
    let app = build_application(
        &pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[("ADMIN_BOOTSTRAP_ENABLED", "false")]),
    )
    .await;

    let result = app
        .state()
        .services
        .bootstrap
        .bootstrap_admin(&request("blocked", "blocked@example.com"))
        .await
        .unwrap();

    assert!(!result.is_successful);
    assert_eq!(result.message.as_deref(), Some("Bootstrap is disabled"));
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(users, 0);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn an_existing_super_admin_locks_the_route_without_creating_anyone(pool: PgPool) {
    let app = build_application(
        &pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await;
    UserFactory::new(&pool, "Seeded Admin").create().await;

    let result = app
        .state()
        .services
        .bootstrap
        .bootstrap_admin(&request("late admin", "late@example.com"))
        .await
        .unwrap();

    assert!(!result.is_successful);
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(users, 1);
    let flag: bool = sqlx::query_scalar(
        "SELECT admin_bootstrapped FROM system_bootstrap WHERE id = 'singleton'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(flag);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn concurrent_attempts_create_exactly_one_admin(pool: PgPool) {
    let app = build_application(
        &pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await;
    let service = app.state().services.bootstrap.clone();

    let attempts = (0..4).map(|index| {
        let service = service.clone();
        async move {
            service
                .bootstrap_admin(&request(
                    &format!("racer {index}"),
                    &format!("racer{index}@example.com"),
                ))
                .await
                .unwrap()
                .is_successful
        }
    });
    let outcomes = futures_util::future::join_all(attempts).await;

    assert_eq!(outcomes.iter().filter(|success| **success).count(), 1);
    let admins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(admins, 1);
}
