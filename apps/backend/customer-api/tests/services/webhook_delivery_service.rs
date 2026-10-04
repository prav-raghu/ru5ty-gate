#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use customer_api::schemas::CreateWebhookSubscriptionRequest;
use customer_api::services::{RetryOutcome, next_retry_delay_seconds};
use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_types::WebhookEventType;
use ru5ty_gate_utilities::WebhookSignatureService;
use serde_json::{Map, Value};
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::common::{RecordingEmailSender, UserFactory, build_application, live_redis};

#[derive(Clone)]
struct Receiver {
    status: StatusCode,
    received: Arc<Mutex<Vec<(HeaderMap, String)>>>,
}

async fn capture(State(receiver): State<Receiver>, headers: HeaderMap, body: String) -> StatusCode {
    receiver.received.lock().unwrap().push((headers, body));
    receiver.status
}

async fn start_receiver(status: StatusCode) -> (String, Receiver) {
    let receiver = Receiver {
        status,
        received: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/hook", post(capture))
        .with_state(receiver.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}/hook"), receiver)
}

fn data() -> Map<String, Value> {
    let mut data = Map::new();
    data.insert("userId".to_owned(), Value::String("abc".to_owned()));
    data
}

#[test]
fn retry_delay_doubles_and_is_capped_at_one_hour() {
    assert_eq!(next_retry_delay_seconds(1), 60);
    assert_eq!(next_retry_delay_seconds(2), 120);
    assert_eq!(next_retry_delay_seconds(3), 240);
    assert_eq!(next_retry_delay_seconds(10), 3600);
}

async fn subscribe(
    pool: &PgPool,
    url: &str,
    retry_count: i32,
) -> (Uuid, customer_api::types::AppState) {
    let app = build_application(pool, RecordingEmailSender::new(true), live_redis().await).await;
    let owner = UserFactory::new(pool, "Hook Owner").create().await;
    app.state()
        .services
        .webhook_subscription
        .create_subscription(
            &CreateWebhookSubscriptionRequest {
                url: url.to_owned(),
                secret: Some("s".repeat(40)),
                events: vec!["user.created".to_owned()],
                retry_count: Some(retry_count),
                timeout_seconds: Some(5),
            },
            owner,
        )
        .await
        .unwrap();
    (owner, app.state().clone())
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn publishing_an_event_delivers_a_signed_payload(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::OK).await;
    let (_, state) = subscribe(&pool, &url, 3).await;

    state
        .services
        .webhook_delivery
        .publish_event(WebhookEventType::UserCreated, data())
        .await
        .unwrap();

    let received = receiver.received.lock().unwrap().clone();
    assert_eq!(received.len(), 1);
    let (headers, body) = &received[0];
    let signature = headers["x-webhook-signature"].to_str().unwrap();
    assert!(WebhookSignatureService.verify_signature(body, signature, &"s".repeat(40)));
    let payload: Value = serde_json::from_str(body).unwrap();
    assert_eq!(payload["event"], "user.created");
    assert_eq!(payload["data"]["userId"], "abc");
    let (status, attempts): (String, i32) =
        sqlx::query_as("SELECT status, attempt_count FROM webhook_deliveries")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "delivered");
    assert_eq!(attempts, 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn events_are_only_sent_to_subscribed_active_endpoints(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::OK).await;
    let (_, state) = subscribe(&pool, &url, 3).await;

    state
        .services
        .webhook_delivery
        .publish_event(WebhookEventType::PaymentFailed, data())
        .await
        .unwrap();

    assert!(receiver.received.lock().unwrap().is_empty());
    let deliveries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webhook_deliveries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(deliveries, 0);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn failures_are_retried_with_backoff_then_marked_failed(pool: PgPool) {
    let (url, receiver) = start_receiver(StatusCode::INTERNAL_SERVER_ERROR).await;
    let (owner, state) = subscribe(&pool, &url, 2).await;
    let service = &state.services.webhook_delivery;

    service
        .publish_event(WebhookEventType::UserCreated, data())
        .await
        .unwrap();
    let (status, attempts, next_retry): (String, i32, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT status, attempt_count, next_retry_at FROM webhook_deliveries")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "retrying");
    assert_eq!(attempts, 1);
    assert!(next_retry.unwrap() > chrono::Utc::now());

    service.process_deliveries().await.unwrap();
    assert_eq!(receiver.received.lock().unwrap().len(), 1);

    let delivery_id: Uuid = sqlx::query_scalar("SELECT id FROM webhook_deliveries")
        .fetch_one(&pool)
        .await
        .unwrap();
    let outcome = service
        .retry_failed_delivery(delivery_id, owner)
        .await
        .unwrap();
    let (status, attempts, next_retry): (String, i32, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT status, attempt_count, next_retry_at FROM webhook_deliveries")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(outcome, RetryOutcome::Started);
    assert_eq!(status, "failed");
    assert_eq!(attempts, 2);
    assert!(next_retry.is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn retry_is_scoped_to_the_owner_and_refuses_delivered_rows(pool: PgPool) {
    let (url, _receiver) = start_receiver(StatusCode::OK).await;
    let (owner, state) = subscribe(&pool, &url, 3).await;
    let service = &state.services.webhook_delivery;
    let intruder = UserFactory::new(&pool, "Intruder").create().await;
    service
        .publish_event(WebhookEventType::UserCreated, data())
        .await
        .unwrap();
    let delivery_id: Uuid = sqlx::query_scalar("SELECT id FROM webhook_deliveries")
        .fetch_one(&pool)
        .await
        .unwrap();

    let foreign = service
        .retry_failed_delivery(delivery_id, intruder)
        .await
        .unwrap();
    let delivered = service
        .retry_failed_delivery(delivery_id, owner)
        .await
        .unwrap();
    let missing = service
        .retry_failed_delivery(Uuid::new_v4(), owner)
        .await
        .unwrap();

    assert_eq!(foreign, RetryOutcome::NotFound);
    assert_eq!(delivered, RetryOutcome::AlreadyDelivered);
    assert_eq!(missing, RetryOutcome::NotFound);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn unreachable_endpoints_record_the_error(pool: PgPool) {
    let (_, state) = subscribe(&pool, "http://127.0.0.1:1/hook", 1).await;

    state
        .services
        .webhook_delivery
        .publish_event(WebhookEventType::UserCreated, data())
        .await
        .unwrap();

    let (status, error): (String, Option<String>) =
        sqlx::query_as("SELECT status, error_message FROM webhook_deliveries")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(error.is_some());
}
