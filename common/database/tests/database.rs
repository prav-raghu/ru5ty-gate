#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use ru5ty_gate_config::EnvReader;
use ru5ty_gate_database::{
    DatabaseConfig, DatabaseError, PgPool, SeedAdmin, seed_admin, seed_all, seed_roles,
    seed_user_statuses,
};

#[test]
fn config_reads_url_with_defaults() {
    let config =
        DatabaseConfig::from_env(&EnvReader::from_pairs([("DATABASE_URL", "postgres://x")]))
            .unwrap();

    assert_eq!(config.url, "postgres://x");
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.acquire_timeout, Duration::from_secs(10));
}

#[test]
fn config_requires_database_url() {
    assert!(DatabaseConfig::from_env(&EnvReader::default()).is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn seeding_roles_and_statuses_is_idempotent(pool: PgPool) {
    seed_roles(&pool).await.unwrap();
    seed_roles(&pool).await.unwrap();
    seed_user_statuses(&pool).await.unwrap();
    seed_user_statuses(&pool).await.unwrap();

    let roles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
        .fetch_one(&pool)
        .await
        .unwrap();
    let statuses: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_statuses")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(roles, 4);
    assert_eq!(statuses, 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_is_created_once_and_never_overwritten(pool: PgPool) {
    seed_all(&pool, None).await.unwrap();
    let admin = SeedAdmin {
        email: "admin@example.com".to_owned(),
        username: "admin".to_owned(),
        password: "correct horse battery".to_owned(),
    };

    assert!(seed_admin(&pool, &admin).await.unwrap());
    let second = SeedAdmin {
        password: "different".to_owned(),
        ..admin.clone()
    };
    assert!(!seed_admin(&pool, &second).await.unwrap());

    let hash: String = sqlx::query_scalar("SELECT password FROM users WHERE email = $1")
        .bind("admin@example.com")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(bcrypt::verify("correct horse battery", &hash).unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn updated_at_is_maintained_by_trigger(pool: PgPool) {
    seed_roles(&pool).await.unwrap();
    let before: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM roles WHERE name = 'Support'")
            .fetch_one(&pool)
            .await
            .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    sqlx::query("UPDATE roles SET is_active = FALSE WHERE name = 'Support'")
        .execute(&pool)
        .await
        .unwrap();
    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM roles WHERE name = 'Support'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(after > before);
}

#[sqlx::test(migrations = "./migrations")]
async fn unique_violations_are_recognised_on_database_errors(pool: PgPool) {
    seed_roles(&pool).await.unwrap();

    let duplicate = sqlx::query("INSERT INTO roles (name) VALUES ('Support')")
        .execute(&pool)
        .await
        .unwrap_err();

    assert!(DatabaseError::from(duplicate).is_unique_violation());
    assert!(!DatabaseError::SeedPrerequisite("role".to_owned()).is_unique_violation());
    assert!(!DatabaseError::from(sqlx::Error::RowNotFound).is_unique_violation());
}
