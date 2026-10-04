#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use customer_api::schemas::{CreateWebhookSubscriptionRequest, UpdateWebhookSubscriptionRequest};
use customer_api::services::WebhookSubscriptionService;
use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_webhooks::TargetPolicy;
use uuid::Uuid;

use crate::common::{RecordingEmailSender, UserFactory, build_application, live_redis};

fn create_request() -> CreateWebhookSubscriptionRequest {
    CreateWebhookSubscriptionRequest {
        url: "https://example.com/hook".to_owned(),
        secret: None,
        events: vec!["user.created".to_owned()],
        retry_count: None,
        timeout_seconds: None,
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn create_applies_defaults_generates_a_secret_and_records_the_actor(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.webhook_subscription;
    let owner = UserFactory::new(&pool, "Hook Owner").create().await;

    let created = service
        .create_subscription(&create_request(), owner)
        .await
        .unwrap()
        .data
        .unwrap();

    assert_eq!(created.retry_count, 3);
    assert_eq!(created.timeout_seconds, 30);
    assert_eq!(created.secret.len(), 64);
    assert!(created.is_active);
    assert_eq!(
        created.created_by.as_deref(),
        Some(owner.to_string().as_str())
    );
    assert_eq!(
        created.modified_by.as_deref(),
        Some(owner.to_string().as_str())
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn subscriptions_are_isolated_per_owner(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.webhook_subscription;
    let owner = UserFactory::new(&pool, "Owner One").create().await;
    let intruder = UserFactory::new(&pool, "Owner Two").create().await;
    let created = service
        .create_subscription(&create_request(), owner)
        .await
        .unwrap()
        .data
        .unwrap();

    let stolen_read = service
        .get_subscription(created.id, intruder)
        .await
        .unwrap();
    let stolen_update = service
        .update_subscription(
            created.id,
            intruder,
            &UpdateWebhookSubscriptionRequest {
                is_active: Some(false),
                ..UpdateWebhookSubscriptionRequest::default()
            },
        )
        .await
        .unwrap();
    let stolen_delete = service
        .delete_subscription(created.id, intruder)
        .await
        .unwrap();
    let stolen_secret = service
        .regenerate_secret(created.id, intruder)
        .await
        .unwrap();
    let stolen_deliveries = service
        .get_deliveries(created.id, intruder, 50)
        .await
        .unwrap();
    let own_list = service.list_subscriptions(owner, None).await.unwrap();
    let other_list = service.list_subscriptions(intruder, None).await.unwrap();

    for result in [&stolen_read, &stolen_update, &stolen_secret] {
        assert!(!result.is_successful);
        assert_eq!(result.message.as_deref(), Some("Subscription not found"));
    }
    assert!(!stolen_delete.is_successful);
    assert!(!stolen_deliveries.is_successful);
    assert_eq!(own_list.data.unwrap().len(), 1);
    assert!(other_list.data.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn update_changes_only_the_provided_fields(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.webhook_subscription;
    let owner = UserFactory::new(&pool, "Updater").create().await;
    let created = service
        .create_subscription(&create_request(), owner)
        .await
        .unwrap()
        .data
        .unwrap();

    let updated = service
        .update_subscription(
            created.id,
            owner,
            &UpdateWebhookSubscriptionRequest {
                is_active: Some(false),
                retry_count: Some(7),
                ..UpdateWebhookSubscriptionRequest::default()
            },
        )
        .await
        .unwrap()
        .data
        .unwrap();

    assert!(!updated.is_active);
    assert_eq!(updated.retry_count, 7);
    assert_eq!(updated.url, created.url);
    assert_eq!(updated.secret, created.secret);
    assert_eq!(updated.events, created.events);
    let active_only = service.list_subscriptions(owner, Some(true)).await.unwrap();
    let inactive_only = service
        .list_subscriptions(owner, Some(false))
        .await
        .unwrap();
    assert!(active_only.data.unwrap().is_empty());
    assert_eq!(inactive_only.data.unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn regenerate_secret_rotates_the_secret_and_delete_removes_the_row(pool: PgPool) {
    let app = build_application(&pool, RecordingEmailSender::new(true), live_redis().await).await;
    let service = &app.state().services.webhook_subscription;
    let owner = UserFactory::new(&pool, "Rotator").create().await;
    let created = service
        .create_subscription(&create_request(), owner)
        .await
        .unwrap()
        .data
        .unwrap();

    let rotated = service
        .regenerate_secret(created.id, owner)
        .await
        .unwrap()
        .data
        .unwrap();
    let deleted = service
        .delete_subscription(created.id, owner)
        .await
        .unwrap();
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webhook_subscriptions")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_ne!(rotated.secret, created.secret);
    assert!(deleted.is_successful);
    assert_eq!(remaining, 0);
    assert!(
        !service
            .delete_subscription(Uuid::new_v4(), owner)
            .await
            .unwrap()
            .is_successful
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn public_only_policy_rejects_private_targets_on_create_and_update(pool: PgPool) {
    crate::common::seed(&pool).await;
    let owner = UserFactory::new(&pool, "Hook Owner").create().await;
    let service =
        WebhookSubscriptionService::new(pool.clone()).with_target_policy(TargetPolicy::PublicOnly);
    let mut private = create_request();
    private.url = "http://169.254.169.254/latest".to_owned();
    let mut public = create_request();
    public.url = "https://93.184.216.34/hook".to_owned();

    let rejected = service.create_subscription(&private, owner).await.unwrap();
    let created = service.create_subscription(&public, owner).await.unwrap();
    let update = UpdateWebhookSubscriptionRequest {
        url: Some("http://10.0.0.8/hook".to_owned()),
        ..UpdateWebhookSubscriptionRequest::default()
    };
    let rejected_update = service
        .update_subscription(created.data.unwrap().id, owner, &update)
        .await
        .unwrap();

    assert!(!rejected.is_successful);
    assert!(!rejected_update.is_successful);
}
