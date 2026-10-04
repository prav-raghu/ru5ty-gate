#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Router;
use axum::routing::{get, post};
use ru5ty_gate_central_client::{CentralClient, ClientConfig, IdentifierPolicy};
use ru5ty_gate_heartbeat::{
    HeartbeatTaskConfig, SyncStatus, SyncTaskConfig, spawn_heartbeat_task, spawn_policy_task,
    spawn_sync_task,
};
use ru5ty_gate_session_store::{SessionStore, StoredPolicy, SyncEventKind};
use secrecy::SecretString;
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

fn sync_config() -> SyncTaskConfig {
    SyncTaskConfig {
        interval: Duration::from_millis(20),
        batch_size: 10,
        max_pending: 1000,
        retain_synced_secs: 86_400,
    }
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

async fn enqueue(store: &SessionStore, payload: serde_json::Value) {
    store
        .enqueue_event(SyncEventKind::SessionStart, payload, 1)
        .await
        .unwrap();
}

#[tokio::test]
async fn sync_task_marks_events_synced_once_the_central_platform_accepts_them() {
    let router = Router::new().route(
        "/v1/venues/venue-1/sync",
        post(|| async { axum::Json(serde_json::json!({ "accepted": 1 })) }),
    );
    let base_url = serve(router).await;
    let store = SessionStore::open_in_memory().unwrap();
    enqueue(&store, serde_json::json!({})).await;
    let status = SyncStatus::new();

    let handle = spawn_sync_task(
        store.clone(),
        client(base_url),
        IdentifierPolicy::raw(),
        status.clone(),
        sync_config(),
    );

    assert!(wait_for_pending_count(&store, 0).await);
    assert!(status.last_ok().is_some());
    handle.abort();
}

#[tokio::test]
async fn sync_task_keeps_events_queued_while_the_central_platform_is_unreachable() {
    let store = SessionStore::open_in_memory().unwrap();
    enqueue(&store, serde_json::json!({})).await;
    let status = SyncStatus::new();

    let handle = spawn_sync_task(
        store.clone(),
        client("http://127.0.0.1:1".to_owned()),
        IdentifierPolicy::raw(),
        status.clone(),
        sync_config(),
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    handle.abort();

    assert_eq!(store.pending_event_count().await.unwrap(), 1);
    assert!(status.last_ok().is_none());
}

#[tokio::test]
async fn sync_task_sends_hashed_identifiers_when_raw_identifiers_are_disabled() {
    let seen: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let recorder = Arc::clone(&seen);
    let router = Router::new().route(
        "/v1/venues/venue-1/sync",
        post(move |axum::Json(body): axum::Json<serde_json::Value>| {
            let recorder = Arc::clone(&recorder);
            async move {
                *recorder.lock().unwrap() = Some(body);
                axum::Json(serde_json::json!({ "accepted": 1 }))
            }
        }),
    );
    let base_url = serve(router).await;
    let store = SessionStore::open_in_memory().unwrap();
    enqueue(
        &store,
        serde_json::json!({"mac": "aa:bb:cc:dd:ee:ff", "client_ip": "10.0.0.5", "venue": "venue-1"}),
    )
    .await;

    let handle = spawn_sync_task(
        store.clone(),
        client(base_url),
        IdentifierPolicy::hashed(SecretString::from("0123456789abcdef".to_owned())),
        SyncStatus::new(),
        sync_config(),
    );

    assert!(wait_for_pending_count(&store, 0).await);
    handle.abort();
    let body = seen.lock().unwrap().clone().unwrap();
    let payload = &body["events"][0]["payload"];
    assert!(payload.get("mac").is_none());
    assert!(payload.get("client_ip").is_none());
    assert_eq!(payload["mac_hash"].as_str().unwrap().len(), 64);
    assert_eq!(payload["venue"], "venue-1");
}

#[tokio::test]
async fn sync_task_drops_the_oldest_events_beyond_the_pending_cap() {
    let store = SessionStore::open_in_memory().unwrap();
    for i in 0..6 {
        enqueue(&store, serde_json::json!({"i": i})).await;
    }

    let handle = spawn_sync_task(
        store.clone(),
        client("http://127.0.0.1:1".to_owned()),
        IdentifierPolicy::raw(),
        SyncStatus::new(),
        SyncTaskConfig {
            max_pending: 2,
            ..sync_config()
        },
    );

    assert!(wait_for_pending_count(&store, 2).await);
    handle.abort();
}

#[tokio::test]
async fn policy_task_caches_the_central_policy_locally() {
    let router = Router::new().route(
        "/v1/venues/venue-1/policy",
        get(|| async {
            axum::Json(serde_json::json!({
                "session_duration_secs": 900,
                "redirect_url": "https://venue.example/landing",
            }))
        }),
    );
    let base_url = serve(router).await;
    let store = SessionStore::open_in_memory().unwrap();

    let handle = spawn_policy_task(store.clone(), client(base_url), Duration::from_millis(20));

    let mut cached = None;
    for _ in 0..100 {
        cached = store.load_policy().await.unwrap();
        if cached.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    handle.abort();
    assert_eq!(
        cached,
        Some(StoredPolicy {
            session_duration_secs: 900,
            redirect_url: Some("https://venue.example/landing".to_owned()),
        })
    );
}

#[tokio::test]
async fn heartbeat_task_reports_queue_depth_and_the_last_successful_sync() {
    let seen: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let recorder = Arc::clone(&seen);
    let router = Router::new().route(
        "/v1/venues/venue-1/heartbeat",
        post(move |axum::Json(body): axum::Json<serde_json::Value>| {
            let recorder = Arc::clone(&recorder);
            async move {
                *recorder.lock().unwrap() = Some(body);
            }
        }),
    );
    let base_url = serve(router).await;
    let store = SessionStore::open_in_memory().unwrap();
    enqueue(&store, serde_json::json!({})).await;
    let status = SyncStatus::new();
    status.mark_ok(1234);

    let handle = spawn_heartbeat_task(
        store,
        client(base_url),
        status,
        HeartbeatTaskConfig {
            venue_id: "venue-1".to_owned(),
            agent_version: "1.2.3".to_owned(),
            interval: Duration::from_millis(20),
            process_start: Instant::now(),
        },
    );

    let mut body = None;
    for _ in 0..100 {
        body = seen.lock().unwrap().clone();
        if body.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    handle.abort();
    let body = body.unwrap();
    assert_eq!(body["pending_events"], 1);
    assert_eq!(body["last_sync_ok_at"], 1234);
    assert_eq!(body["agent_version"], "1.2.3");
}
