#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use axum::http::StatusCode;
use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_utilities::WebhookSignatureService;
use ru5ty_gate_webhooks::WebhookDeliveryService;
use schedule_api::jobs::{ScheduledJob, WebhookProcessorJob};

use crate::common::{delivery_state, seed_delivery, start_receiver};

fn job(pool: &PgPool) -> WebhookProcessorJob {
    WebhookProcessorJob::new(
        WebhookDeliveryService::new(pool.clone()),
        Duration::from_secs(60),
    )
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn pending_deliveries_are_sent_signed_and_marked_delivered(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::OK).await;
    let id = seed_delivery(&pool, &url, "pending", 3).await;

    job(&pool).run().await.unwrap();

    let received = receiver.received.lock().unwrap().clone();
    assert_eq!(received.len(), 1);
    let (headers, body) = &received[0];
    assert!(WebhookSignatureService.verify_signature(
        body,
        headers["x-webhook-signature"].to_str().unwrap(),
        &"s".repeat(40)
    ));
    assert_eq!(delivery_state(&pool, id).await, ("delivered".to_owned(), 1));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn failures_are_retried_later_and_eventually_marked_failed(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::INTERNAL_SERVER_ERROR).await;
    let id = seed_delivery(&pool, &url, "pending", 2).await;

    job(&pool).run().await.unwrap();
    assert_eq!(delivery_state(&pool, id).await, ("retrying".to_owned(), 1));
    job(&pool).run().await.unwrap();
    assert_eq!(
        receiver.received.lock().unwrap().len(),
        1,
        "retry is not due yet"
    );

    sqlx::query("UPDATE webhook_deliveries SET next_retry_at = NOW() - INTERVAL '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    job(&pool).run().await.unwrap();

    assert_eq!(receiver.received.lock().unwrap().len(), 2);
    assert_eq!(delivery_state(&pool, id).await, ("failed".to_owned(), 2));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn delivered_and_failed_rows_are_never_resent(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::OK).await;
    seed_delivery(&pool, &url, "delivered", 3).await;
    seed_delivery(&pool, &url, "failed", 3).await;

    job(&pool).run().await.unwrap();

    assert!(receiver.received.lock().unwrap().is_empty());
}
