#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use admin_api::schemas::{BulkCreateUserItem, BulkUpdateStatusItem};
use ru5ty_gate_database::sqlx::{self, PgPool};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::common::{
    RecordingEmailSender, UserFactory, build_application, config_with, live_redis, unique_name,
};

async fn app(pool: &PgPool) -> admin_api::application::Application {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await
}

fn item(name: &str, email: &str) -> BulkCreateUserItem {
    BulkCreateUserItem {
        email: email.to_owned(),
        name: name.to_owned(),
        password: "CorrectHorse1".to_owned(),
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn bulk_create_reports_per_item_results_and_continues_after_conflicts(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.batch;
    let (actor, _) = UserFactory::new(&pool, &unique_name("Batcher"))
        .create()
        .await;

    let summary = service
        .bulk_create_users(
            &[
                item("Bulk One", "bulk1@example.com"),
                item("Bulk One", "bulk1b@example.com"),
                item("Bulk Two", "bulk2@example.com"),
            ],
            actor,
        )
        .await
        .unwrap();

    assert_eq!(summary.total, 3);
    assert_eq!(summary.successful, 2);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.results[0].id, "user-0");
    assert_eq!(
        summary.results[1].error.as_deref(),
        Some("Username or email already exists")
    );
    let (role, status, created_by): (String, String, String) = sqlx::query_as(
        "SELECT r.name, s.name, u.created_by FROM users u \
         JOIN roles r ON r.id = u.role_id JOIN user_statuses s ON s.id = u.user_status_id \
         WHERE u.email = 'bulk2@example.com'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(role, "Chat User");
    assert_eq!(status, "Pending Verification");
    assert_eq!(created_by, actor.to_string());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn bulk_status_update_isolates_failures_with_savepoints(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.batch;
    let (actor, _) = UserFactory::new(&pool, &unique_name("Updater"))
        .create()
        .await;
    let (first, _) = UserFactory::new(&pool, &unique_name("Target One"))
        .create()
        .await;
    let (second, _) = UserFactory::new(&pool, &unique_name("Target Two"))
        .create()
        .await;

    let summary = service
        .bulk_update_user_status(
            &[
                BulkUpdateStatusItem {
                    user_id: first,
                    status: "Verified".to_owned(),
                },
                BulkUpdateStatusItem {
                    user_id: Uuid::new_v4(),
                    status: "Verified".to_owned(),
                },
                BulkUpdateStatusItem {
                    user_id: second,
                    status: "Nonexistent".to_owned(),
                },
                BulkUpdateStatusItem {
                    user_id: second,
                    status: "Offline".to_owned(),
                },
            ],
            actor,
        )
        .await
        .unwrap();

    assert_eq!((summary.successful, summary.failed), (2, 2));
    assert_eq!(summary.results[1].error.as_deref(), Some("User not found"));
    assert_eq!(
        summary.results[2].error.as_deref(),
        Some("User status 'Nonexistent' not found")
    );
    let statuses: Vec<String> = sqlx::query_scalar(
        "SELECT s.name FROM users u JOIN user_statuses s ON s.id = u.user_status_id \
         WHERE u.id = ANY($1) ORDER BY u.username",
    )
    .bind(vec![first, second])
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(statuses, vec!["Verified".to_owned(), "Offline".to_owned()]);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn bulk_delete_is_all_or_nothing(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.batch;
    let (actor, _) = UserFactory::new(&pool, &unique_name("Deleter"))
        .create()
        .await;
    let (victim, _) = UserFactory::new(&pool, &unique_name("Victim"))
        .create()
        .await;

    let failed = service
        .bulk_delete_users(&[victim, Uuid::new_v4()], actor)
        .await;
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(victim)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(failed.is_err());
    assert_eq!(remaining, 1);

    let ok = service.bulk_delete_users(&[victim], actor).await.unwrap();
    assert_eq!((ok.total, ok.successful, ok.failed), (1, 1, 0));
    let gone: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(victim)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(gone, 0);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn bulk_delete_refuses_duplicates_and_self_deletion(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.batch;
    let (actor, _) = UserFactory::new(&pool, &unique_name("Self Deleter"))
        .create()
        .await;
    let (other, _) = UserFactory::new(&pool, &unique_name("Other"))
        .create()
        .await;

    assert!(
        service
            .bulk_delete_users(&[other, other], actor)
            .await
            .is_err()
    );
    assert!(
        service
            .bulk_delete_users(&[other, actor], actor)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn custom_batches_echo_their_items_with_stable_ids(pool: PgPool) {
    let app = app(&pool).await;
    let mut first = Map::new();
    first.insert("a".to_owned(), json!(1));

    let summary = app
        .state()
        .services
        .batch
        .execute_custom_batch(&[first.clone(), first]);

    assert_eq!(
        (summary.total, summary.successful, summary.failed),
        (2, 2, 0)
    );
    assert_eq!(summary.results[1].id, "item-1");
    assert_eq!(
        summary.results[0].data,
        Some(Value::Object({
            let mut expected = Map::new();
            expected.insert("a".to_owned(), json!(1));
            expected
        }))
    );
}
