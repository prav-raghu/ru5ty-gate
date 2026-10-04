#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use axum::Router;
use axum::routing::post;
use ru5ty_gate_central_client::{CentralClient, ClientConfig};
use ru5ty_gate_heartbeat::spawn_sync_task;
use ru5ty_gate_session_store::{SessionStore, SyncEventKind};
use tokio::net::TcpListener;

async fn serve(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{address}")
}

fn client(base_url: String) -> CentralClient {
    CentralClient::new(ClientConfig {
        base_url,
        api_key: None,
        timeout: Duration::from_secs(2),
        venue_id: "venue-1".to_owned(),
    })
    .unwrap()
}

async fn wait_for_pending_count(store: &SessionStore, expected: i64) -> bool {
    for _ in 0..100 {
        if store.pending_event_count().await.unwrap() == expected {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

#[tokio::test]
async fn sync_task_marks_events_synced_once_the_central_platform_accepts_them() {
    let router = Router::new().route(
        "/v1/venues/venue-1/sync",
        post(|| async { axum::Json(serde_json::json!({ "accepted": 1 })) }),
    );
    let base_url = serve(router).await;
    let store = SessionStore::open_in_memory().unwrap();
    store
        .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({}), 1)
        .await
        .unwrap();

    let handle = spawn_sync_task(
        store.clone(),
        client(base_url),
        Duration::from_millis(20),
        10,
    );

    assert!(wait_for_pending_count(&store, 0).await);
    handle.abort();
}

#[tokio::test]
async fn sync_task_keeps_events_queued_while_the_central_platform_is_unreachable() {
    let store = SessionStore::open_in_memory().unwrap();
    store
        .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({}), 1)
        .await
        .unwrap();

    let handle = spawn_sync_task(
        store.clone(),
        client("http://127.0.0.1:1".to_owned()),
        Duration::from_millis(20),
        10,
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.abort();

    assert_eq!(store.pending_event_count().await.unwrap(), 1);
}
