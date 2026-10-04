#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_session_store::{
    Session, SessionStore, StoredPolicy, SyncEventKind, build_unix, clock_is_trusted, unix_now,
};

fn session(mac: &str, expires_at: i64) -> Session {
    Session {
        mac: mac.to_owned(),
        token: "tok-1".to_owned(),
        venue: "venue-1".to_owned(),
        granted_at: 0,
        expires_at,
        redirect_url: None,
        clock_untrusted: false,
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

#[tokio::test]
async fn macs_are_matched_case_insensitively() {
    let store = SessionStore::open_in_memory().unwrap();
    store
        .upsert_session(session("AA:BB:CC:DD:EE:FF", 1000))
        .await
        .unwrap();

    let got = store.get_session("aa:bb:cc:dd:ee:ff").await.unwrap();

    assert_eq!(got.unwrap().mac, "aa:bb:cc:dd:ee:ff");
}

#[tokio::test]
async fn sweep_expired_removes_sessions_and_queues_session_end_events() {
    let store = SessionStore::open_in_memory().unwrap();
    store.upsert_session(session("expired", 100)).await.unwrap();
    store
        .upsert_session(session("active", 9_999_999_999))
        .await
        .unwrap();

    let removed = store.sweep_expired(500).await.unwrap();

    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].mac, "expired");
    assert!(store.get_session("expired").await.unwrap().is_none());
    assert!(store.get_session("active").await.unwrap().is_some());
    let events = store.pending_events(10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, SyncEventKind::SessionEnd);
    assert_eq!(events[0].payload["reason"], "expired");
    assert_eq!(events[0].payload["mac"], "expired");
    assert_eq!(events[0].payload["ended_at"], 500);
}

#[tokio::test]
async fn sweep_expired_ignores_sessions_granted_under_an_untrusted_clock() {
    let store = SessionStore::open_in_memory().unwrap();
    let mut untrusted = session("untrusted", 100);
    untrusted.clock_untrusted = true;
    store.upsert_session(untrusted).await.unwrap();

    let removed = store.sweep_expired(500).await.unwrap();

    assert!(removed.is_empty());
    assert!(store.get_session("untrusted").await.unwrap().is_some());
}

#[tokio::test]
async fn rebase_untrusted_sessions_restarts_the_remaining_duration_from_now() {
    let store = SessionStore::open_in_memory().unwrap();
    let mut untrusted = session("untrusted", 3700);
    untrusted.granted_at = 100;
    untrusted.clock_untrusted = true;
    store.upsert_session(untrusted).await.unwrap();

    let rebased = store.rebase_untrusted_sessions(10_000).await.unwrap();

    assert_eq!(rebased, 1);
    let got = store.get_session("untrusted").await.unwrap().unwrap();
    assert_eq!(got.granted_at, 10_000);
    assert_eq!(got.expires_at, 13_600);
    assert!(!got.clock_untrusted);
}

#[tokio::test]
async fn end_session_removes_the_session_and_queues_an_event_once() {
    let store = SessionStore::open_in_memory().unwrap();
    store
        .upsert_session(session("AA:BB:CC:DD:EE:FF", 9_999_999_999))
        .await
        .unwrap();

    let ended = store
        .end_session("aa:bb:cc:dd:ee:ff", "client_deauth", 50)
        .await
        .unwrap();
    let again = store
        .end_session("aa:bb:cc:dd:ee:ff", "client_deauth", 60)
        .await
        .unwrap();

    assert!(ended.is_some());
    assert!(again.is_none());
    let events = store.pending_events(10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["reason"], "client_deauth");
}

#[tokio::test]
async fn prune_synced_only_removes_old_synced_events() {
    let store = SessionStore::open_in_memory().unwrap();
    let old = store
        .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({}), 1)
        .await
        .unwrap();
    let recent = store
        .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({}), 2)
        .await
        .unwrap();
    store.mark_synced(vec![old], 10).await.unwrap();
    store.mark_synced(vec![recent], 1000).await.unwrap();
    store
        .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({}), 3)
        .await
        .unwrap();

    let pruned = store.prune_synced(500).await.unwrap();

    assert_eq!(pruned, 1);
    assert_eq!(store.pending_event_count().await.unwrap(), 1);
}

#[tokio::test]
async fn cap_pending_events_drops_the_oldest_pending_rows() {
    let store = SessionStore::open_in_memory().unwrap();
    for i in 0..5 {
        store
            .enqueue_event(SyncEventKind::SessionStart, serde_json::json!({"i": i}), i)
            .await
            .unwrap();
    }

    let dropped = store.cap_pending_events(2).await.unwrap();

    assert_eq!(dropped, 3);
    let pending = store.pending_events(10).await.unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].payload["i"], 3);
    assert_eq!(pending[1].payload["i"], 4);
}

#[tokio::test]
async fn meta_values_round_trip_and_overwrite() {
    let store = SessionStore::open_in_memory().unwrap();

    assert!(store.get_meta("policy").await.unwrap().is_none());
    store.set_meta("policy", "one").await.unwrap();
    store.set_meta("policy", "two").await.unwrap();

    assert_eq!(
        store.get_meta("policy").await.unwrap().as_deref(),
        Some("two")
    );
}

#[test]
fn the_clock_is_trusted_only_after_the_build_time() {
    assert!(build_unix() > 1_700_000_000);
    assert!(clock_is_trusted(unix_now()));
    assert!(!clock_is_trusted(build_unix() - 1));
}

#[tokio::test]
async fn opening_a_version_one_database_adds_the_clock_flag_column() {
    let dir = std::env::temp_dir().join(format!("ru5ty-gate-migrate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v1.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE sessions (
                mac TEXT PRIMARY KEY, token TEXT NOT NULL, venue TEXT NOT NULL,
                granted_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, redirect_url TEXT
            );
            INSERT INTO sessions VALUES ('aa:bb:cc:dd:ee:ff', 't', 'v', 1, 2, NULL);",
        )
        .unwrap();
    }

    let store = SessionStore::open(&path).unwrap();
    let got = store
        .get_session("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap()
        .unwrap();

    assert!(!got.clock_untrusted);
    assert_eq!(got.expires_at, 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn policy_round_trips_with_and_without_a_redirect() {
    let store = SessionStore::open_in_memory().unwrap();
    assert!(store.load_policy().await.unwrap().is_none());

    let with_redirect = StoredPolicy {
        session_duration_secs: 1800,
        redirect_url: Some("https://venue.example/welcome".to_owned()),
    };
    store.save_policy(&with_redirect).await.unwrap();
    assert_eq!(store.load_policy().await.unwrap(), Some(with_redirect));

    let without_redirect = StoredPolicy {
        session_duration_secs: 60,
        redirect_url: None,
    };
    store.save_policy(&without_redirect).await.unwrap();
    assert_eq!(store.load_policy().await.unwrap(), Some(without_redirect));
}
