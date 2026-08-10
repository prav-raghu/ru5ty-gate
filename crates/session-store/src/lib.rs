//! Local, durable session store for the captive portal agent.
//!
//! This is the offline-buffer: active client sessions (MAC, token, expiry,
//! venue) live in a local sqlite file so the agent survives restarts and
//! central-platform outages, and session-lifecycle events are queued here
//! for batched sync once connectivity to the central API returns. Same
//! offline-buffer pattern as the C++/water project, kept consistent on
//! purpose.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};

mod schema;

pub use schema::CURRENT_SCHEMA_VERSION;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("failed to (de)serialize event payload: {0}")]
    Json(#[from] serde_json::Error),
    #[error("blocking task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("failed to create db directory {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unknown sync event kind: {0}")]
    UnknownEventKind(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// A locally-persisted client session, keyed by MAC address.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub mac: String,
    pub token: String,
    pub venue: String,
    /// Unix timestamp (seconds) the session was granted.
    pub granted_at: i64,
    /// Unix timestamp (seconds) the session expires.
    pub expires_at: i64,
    /// Post-auth redirect URL, if the central platform (or local policy)
    /// supplied one.
    pub redirect_url: Option<String>,
}

impl Session {
    pub fn is_expired(&self, now: i64) -> bool {
        now >= self.expires_at
    }
}

/// The kind of a buffered sync event. v1 only tracks session start/end --
/// heartbeats are posted directly, not buffered through this queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEventKind {
    SessionStart,
    SessionEnd,
}

impl SyncEventKind {
    fn as_str(self) -> &'static str {
        match self {
            SyncEventKind::SessionStart => "session_start",
            SyncEventKind::SessionEnd => "session_end",
        }
    }

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "session_start" => Ok(SyncEventKind::SessionStart),
            "session_end" => Ok(SyncEventKind::SessionEnd),
            other => Err(StoreError::UnknownEventKind(other.to_string())),
        }
    }
}

/// A queued session-lifecycle event awaiting sync to the central platform.
#[derive(Debug, Clone)]
pub struct SyncEvent {
    pub id: i64,
    pub kind: SyncEventKind,
    pub payload: serde_json::Value,
    pub created_at: i64,
}

/// Handle to the local sqlite-backed store. Cheap to clone -- clones share
/// the same underlying connection via `Arc<Mutex<_>>`. All operations hop
/// onto a blocking thread pool via `spawn_blocking` since rusqlite is
/// synchronous.
#[derive(Clone)]
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    /// Open (creating if necessary) the sqlite database at `path`, running
    /// schema migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| StoreError::CreateDir {
                    path: parent.display().to_string(),
                    source,
                })?;
            }
        }
        let conn = Connection::open(path)?;
        schema::migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database. Handy for tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    async fn with_conn<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().expect("session store mutex poisoned");
            f(&guard)
        })
        .await?
        .map_err(StoreError::from)
    }

    /// Insert or replace the session for `session.mac`.
    pub async fn upsert_session(&self, session: Session) -> Result<()> {
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO sessions (mac, token, venue, granted_at, expires_at, redirect_url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(mac) DO UPDATE SET
                    token = excluded.token,
                    venue = excluded.venue,
                    granted_at = excluded.granted_at,
                    expires_at = excluded.expires_at,
                    redirect_url = excluded.redirect_url",
                params![
                    session.mac,
                    session.token,
                    session.venue,
                    session.granted_at,
                    session.expires_at,
                    session.redirect_url,
                ],
            )?;
            Ok(())
        })
        .await
    }

    /// Look up a session by client MAC address.
    pub async fn get_session(&self, mac: impl Into<String>) -> Result<Option<Session>> {
        let mac = mac.into();
        self.with_conn(move |conn| {
            conn.query_row(
                "SELECT mac, token, venue, granted_at, expires_at, redirect_url
                 FROM sessions WHERE mac = ?1",
                params![mac],
                |row| {
                    Ok(Session {
                        mac: row.get(0)?,
                        token: row.get(1)?,
                        venue: row.get(2)?,
                        granted_at: row.get(3)?,
                        expires_at: row.get(4)?,
                        redirect_url: row.get(5)?,
                    })
                },
            )
            .optional()
        })
        .await
    }

    /// Remove sessions that expired at or before `now`. Returns the number
    /// removed.
    pub async fn delete_expired(&self, now: i64) -> Result<usize> {
        self.with_conn(move |conn| {
            conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now])
        })
        .await
    }

    /// Count sessions that are not yet expired as of `now`. Used to fill in
    /// the heartbeat payload.
    pub async fn active_session_count(&self, now: i64) -> Result<i64> {
        self.with_conn(move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM sessions WHERE expires_at > ?1",
                params![now],
                |row| row.get(0),
            )
        })
        .await
    }

    /// Queue a session-lifecycle event for later batched sync.
    pub async fn enqueue_event(
        &self,
        kind: SyncEventKind,
        payload: serde_json::Value,
        created_at: i64,
    ) -> Result<i64> {
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO sync_queue (event_type, payload, created_at, synced_at)
                 VALUES (?1, ?2, ?3, NULL)",
                params![kind.as_str(), payload.to_string(), created_at],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await
    }

    /// Fetch up to `limit` events that haven't been synced yet, oldest
    /// first.
    pub async fn pending_events(&self, limit: u32) -> Result<Vec<SyncEvent>> {
        self.with_conn(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, event_type, payload, created_at FROM sync_queue
                 WHERE synced_at IS NULL ORDER BY id ASC LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit], |row| {
                let event_type: String = row.get(1)?;
                let payload_str: String = row.get(2)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    event_type,
                    payload_str,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            let mut events = Vec::new();
            for row in rows {
                let (id, event_type, payload_str, created_at) = row?;
                let kind = SyncEventKind::from_str(&event_type).map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        1,
                        "event_type".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
                events.push(SyncEvent {
                    id,
                    kind,
                    payload,
                    created_at,
                });
            }
            Ok(events)
        })
        .await
    }

    /// Mark the given queued event ids as synced (so they're excluded from
    /// future `pending_events` calls).
    pub async fn mark_synced(&self, ids: Vec<i64>, synced_at: i64) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        self.with_conn(move |conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare("UPDATE sync_queue SET synced_at = ?1 WHERE id = ?2")?;
                for id in &ids {
                    stmt.execute(params![synced_at, id])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
        .await
    }

    /// Count how many events are still waiting to be synced.
    pub async fn pending_event_count(&self) -> Result<i64> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM sync_queue WHERE synced_at IS NULL",
                [],
                |row| row.get(0),
            )
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(mac: &str, expires_at: i64) -> Session {
        Session {
            mac: mac.to_string(),
            token: "tok-1".to_string(),
            venue: "venue-1".to_string(),
            granted_at: 0,
            expires_at,
            redirect_url: None,
        }
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

        assert!(store
            .get_session("00:00:00:00:00:00")
            .await
            .unwrap()
            .is_none());
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

        let pending = store.pending_events(10).await.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(store.pending_event_count().await.unwrap(), 2);

        store.mark_synced(vec![id1, id2], 3).await.unwrap();
        let pending = store.pending_events(10).await.unwrap();
        assert!(pending.is_empty());
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
}
