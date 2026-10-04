#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_database::{
    NewWebhookSubscription, PageRequest, PgPool, Repository, Role, WebhookDelivery,
    WebhookSubscription, WebhookSubscriptionChanges, seed_roles,
};
use uuid::Uuid;

fn new_subscription(url: &str) -> NewWebhookSubscription {
    NewWebhookSubscription {
        url: url.to_owned(),
        secret: "secret".to_owned(),
        events: vec!["user.created".to_owned()],
        retry_count: 3,
        timeout_seconds: 30,
        created_by: "creator".to_owned(),
    }
}

#[test]
fn page_request_clamps_limit_and_offset() {
    let page = PageRequest::new(Some(10_000), Some(-5));

    assert_eq!(page.limit, PageRequest::MAX_LIMIT);
    assert_eq!(page.offset, 0);
    assert_eq!(PageRequest::default().limit, PageRequest::DEFAULT_LIMIT);
}

#[sqlx::test(migrations = "./migrations")]
async fn insert_returns_the_stored_row_and_find_by_id_reads_it_back(pool: PgPool) {
    let created = Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://a"))
        .await
        .unwrap();

    assert_eq!(created.url, "https://a");
    assert_eq!(created.created_by.as_deref(), Some("creator"));
    assert_eq!(created.modified_by.as_deref(), Some("creator"));
    assert!(created.is_active);

    let found = Repository::<WebhookSubscription>::find_by_id(&pool, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, created.id);
    assert!(
        Repository::<WebhookSubscription>::find_by_id(&pool, Uuid::new_v4())
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn update_by_id_replaces_the_listed_columns(pool: PgPool) {
    let created = Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://a"))
        .await
        .unwrap();
    let changes = WebhookSubscriptionChanges {
        url: "https://b".to_owned(),
        secret: "rotated".to_owned(),
        events: vec!["user.updated".to_owned()],
        is_active: false,
        retry_count: 5,
        timeout_seconds: 10,
        modified_by: "editor".to_owned(),
    };

    let updated = Repository::<WebhookSubscription>::update_by_id(&pool, created.id, &changes)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.url, "https://b");
    assert_eq!(updated.secret, "rotated");
    assert_eq!(updated.events, vec!["user.updated".to_owned()]);
    assert!(!updated.is_active);
    assert_eq!(updated.retry_count, 5);
    assert_eq!(updated.created_by.as_deref(), Some("creator"));
    assert_eq!(updated.modified_by.as_deref(), Some("editor"));
    assert!(
        Repository::<WebhookSubscription>::update_by_id(&pool, Uuid::new_v4(), &changes)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn list_count_and_exists_cover_the_table(pool: PgPool) {
    let first = Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://1"))
        .await
        .unwrap();
    Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://2"))
        .await
        .unwrap();
    Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://3"))
        .await
        .unwrap();

    assert_eq!(
        Repository::<WebhookSubscription>::count(&pool)
            .await
            .unwrap(),
        3
    );
    assert!(
        Repository::<WebhookSubscription>::exists(&pool, first.id)
            .await
            .unwrap()
    );
    assert!(
        !Repository::<WebhookSubscription>::exists(&pool, Uuid::new_v4())
            .await
            .unwrap()
    );

    let page = Repository::<WebhookSubscription>::list(&pool, PageRequest::new(Some(2), Some(0)))
        .await
        .unwrap();
    let rest = Repository::<WebhookSubscription>::list(&pool, PageRequest::new(Some(2), Some(2)))
        .await
        .unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(rest.len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_by_id_reports_whether_a_row_was_removed(pool: PgPool) {
    let created = Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://a"))
        .await
        .unwrap();

    assert!(
        Repository::<WebhookSubscription>::delete_by_id(&pool, created.id)
            .await
            .unwrap()
    );
    assert!(
        !Repository::<WebhookSubscription>::delete_by_id(&pool, created.id)
            .await
            .unwrap()
    );
    assert_eq!(
        Repository::<WebhookSubscription>::count(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn deactivate_keeps_the_row_and_records_the_actor(pool: PgPool) {
    let created = Repository::<WebhookSubscription>::insert(&pool, &new_subscription("https://a"))
        .await
        .unwrap();

    assert!(
        Repository::<WebhookSubscription>::deactivate(&pool, created.id, "actor")
            .await
            .unwrap()
    );

    let found = Repository::<WebhookSubscription>::find_by_id(&pool, created.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!found.is_active);
    assert_eq!(found.modified_by.as_deref(), Some("actor"));
    assert!(
        !Repository::<WebhookSubscription>::deactivate(&pool, Uuid::new_v4(), "actor")
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn the_same_repository_serves_other_entities_and_transactions(pool: PgPool) {
    seed_roles(&pool).await.unwrap();

    let roles = Repository::<Role>::list(&pool, PageRequest::default())
        .await
        .unwrap();
    assert_eq!(roles.len(), 4);
    assert_eq!(
        Repository::<WebhookDelivery>::count(&pool).await.unwrap(),
        0
    );

    let mut transaction = pool.begin().await.unwrap();
    Repository::<WebhookSubscription>::insert(&mut *transaction, &new_subscription("https://tx"))
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    assert_eq!(
        Repository::<WebhookSubscription>::count(&pool)
            .await
            .unwrap(),
        0
    );
}
