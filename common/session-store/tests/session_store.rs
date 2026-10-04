#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_session_store::{Session, SessionStore, SyncEventKind};

fn session(mac: &str, expires_at: i64) -> Session {
    Session {
        mac: mac.to_owned(),
        token: "tok-1".to_owned(),
        venue: "venue-1".to_owned(),
        granted_at: 0,
        expires_at,
        redirect_url: None,
    }
}

#[test]
fn is_expired_is_true_at_and_after_the_expiry_instant() {
    let session = session("AA:BB:CC:DD:EE:FF", 1000);

    assert!(!session.is_expired(999));
    assert!(session.is_expired(1000));
    assert!(session.is_expired(1001));
}

#[test]
fn sync_event_kind_round_trips_through_its_string_form() {
    for kind in [SyncEventKind::SessionStart, SyncEventKind::SessionEnd] {
        assert_eq!(kind.as_str().parse::<SyncEventKind>().unwrap(), kind);
    }
    assert!("bogus".parse::<SyncEventKind>().is_err());
}

#[tokio::test]
async fn upsert_and_get_roundtrip() {
    let store = SessionStore::open_in_memory().unwrap();
    store
        .upsert_session(session("AA:BB:CC:DD:EE:FF", 1000))
        .await
        .unwrap();

    let got = store.get_session("AA:BB:CC:DD:EE:FF").await.unwrap();

    assert_eq!(got.unwrap().expires_at, 1000);
    assert!(
        store
            .get_session("00:00:00:00:00:00")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn upsert_replaces_existing_session() {
    let store = SessionStore::open_in_memory().unwrap();
    store
        .upsert_session(session("AA:BB:CC:DD:EE:FF", 1000))
        .await
        .unwrap();
    store
        .upsert_session(session("AA:BB:CC:DD:EE:FF", 2000))
        .await
        .unwrap();

    let got = store
        .get_session("AA:BB:CC:DD:EE:FF")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(got.expires_at, 2000);
}

#[tokio::test]
async fn expired_sessions_are_swept() {
    let store = SessionStore::open_in_memory().unwrap();
    store.upsert_session(session("expired", 100)).await.unwrap();
    store
        .upsert_session(session("active", 9_999_999_999))
        .await
        .unwrap();

    let removed = store.delete_expired(500).await.unwrap();

    assert_eq!(removed, 1);
    assert!(store.get_session("expired").await.unwrap().is_none());
    assert!(store.get_session("active").await.unwrap().is_some());
}

#[tokio::test]
async fn active_session_count_excludes_expired() {
    let store = SessionStore::open_in_memory().unwrap();
    store.upsert_session(session("expired", 100)).await.unwrap();
    store
        .upsert_session(session("active", 9_999_999_999))
        .await
        .unwrap();

    let count = store.active_session_count(500).await.unwrap();

    assert_eq!(count, 1);
}

#[tokio::test]
async fn sync_queue_enqueue_pending_mark_synced() {
    let store = SessionStore::open_in_memory().unwrap();
    let id1 = store
        .enqueue_event(
            SyncEventKind::SessionStart,
            serde_json::json!({"mac": "a"}),
            1,
        )
        .await
        .unwrap();
    let id2 = store
        .enqueue_event(
            SyncEventKind::SessionEnd,
            serde_json::json!({"mac": "b"}),
            2,
        )
        .await
        .unwrap();

    assert_eq!(store.pending_events(10).await.unwrap().len(), 2);
    assert_eq!(store.pending_event_count().await.unwrap(), 2);

    store.mark_synced(vec![id1, id2], 3).await.unwrap();

    assert!(store.pending_events(10).await.unwrap().is_empty());
    assert_eq!(store.pending_event_count().await.unwrap(), 0);
}

#[tokio::test]
async fn pending_events_respects_limit_and_order() {
    let store = SessionStore::open_in_memory().unwrap();
    for i in 0..5 {
        store
            .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({"i": i}), i)
            .await
            .unwrap();
    }

    let pending = store.pending_events(2).await.unwrap();

    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].payload["i"], 0);
    assert_eq!(pending[1].payload["i"], 1);
}

#[tokio::test]
async fn open_creates_missing_parent_directories() {
    let dir = std::env::temp_dir().join(format!("ru5ty-gate-store-{}", std::process::id()));
    let path = dir.join("nested").join("sessions.db");

    let store = SessionStore::open(&path).unwrap();
    store.upsert_session(session("mac", 10)).await.unwrap();

    assert!(path.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
