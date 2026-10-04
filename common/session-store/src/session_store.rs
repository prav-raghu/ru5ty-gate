use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::{Result, Session, StoreError, StoredPolicy, SyncEvent, SyncEventKind, schema};

const SESSION_COLUMNS: &str =
    "mac, token, venue, granted_at, expires_at, redirect_url, clock_untrusted";

fn normalize_mac(mac: &str) -> String {
    mac.to_ascii_lowercase()
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        mac: row.get(0)?,
        token: row.get(1)?,
        venue: row.get(2)?,
        granted_at: row.get(3)?,
        expires_at: row.get(4)?,
        redirect_url: row.get(5)?,
        clock_untrusted: row.get::<_, i64>(6)? != 0,
    })
}

fn enqueue_session_end(
    tx: &Transaction<'_>,
    session: &Session,
    reason: &str,
    now: i64,
) -> rusqlite::Result<()> {
    let payload = serde_json::json!({
        "mac": session.mac,
        "venue": session.venue,
        "reason": reason,
        "granted_at": session.granted_at,
        "ended_at": now,
    });
    tx.execute(
        "INSERT INTO sync_queue (event_type, payload, created_at, synced_at)
         VALUES (?1, ?2, ?3, NULL)",
        params![SyncEventKind::SessionEnd.as_str(), payload.to_string(), now],
    )?;
    Ok(())
}

#[derive(Clone)]
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|source| StoreError::CreateDir {
                path: parent.display().to_string(),
                source,
            })?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
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
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().map_err(|_| StoreError::Poisoned)?;
            f(&guard).map_err(StoreError::from)
        })
        .await?
    }

    pub async fn upsert_session(&self, session: Session) -> Result<()> {
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO sessions
                    (mac, token, venue, granted_at, expires_at, redirect_url, clock_untrusted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(mac) DO UPDATE SET
                    token = excluded.token,
                    venue = excluded.venue,
                    granted_at = excluded.granted_at,
                    expires_at = excluded.expires_at,
                    redirect_url = excluded.redirect_url,
                    clock_untrusted = excluded.clock_untrusted",
                params![
                    normalize_mac(&session.mac),
                    session.token,
                    session.venue,
                    session.granted_at,
                    session.expires_at,
                    session.redirect_url,
                    i64::from(session.clock_untrusted),
                ],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn get_session(&self, mac: impl Into<String>) -> Result<Option<Session>> {
        let mac = normalize_mac(&mac.into());
        self.with_conn(move |conn| {
            conn.query_row(
                &format!("SELECT {SESSION_COLUMNS} FROM sessions WHERE mac = ?1"),
                params![mac],
                session_from_row,
            )
            .optional()
        })
        .await
    }

    pub async fn delete_expired(&self, now: i64) -> Result<usize> {
        self.with_conn(move |conn| {
            conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now])
        })
        .await
    }

    pub async fn sweep_expired(&self, now: i64) -> Result<Vec<Session>> {
        self.with_conn(move |conn| {
            let tx = conn.unchecked_transaction()?;
            let expired = {
                let mut stmt = tx.prepare(&format!(
                    "SELECT {SESSION_COLUMNS} FROM sessions
                     WHERE expires_at <= ?1 AND clock_untrusted = 0 ORDER BY expires_at"
                ))?;
                stmt.query_map(params![now], session_from_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            for session in &expired {
                tx.execute("DELETE FROM sessions WHERE mac = ?1", params![session.mac])?;
                enqueue_session_end(&tx, session, "expired", now)?;
            }
            tx.commit()?;
            Ok(expired)
        })
        .await
    }

    pub async fn end_session(
        &self,
        mac: impl Into<String>,
        reason: impl Into<String>,
        now: i64,
    ) -> Result<Option<Session>> {
        let mac = normalize_mac(&mac.into());
        let reason = reason.into();
        self.with_conn(move |conn| {
            let tx = conn.unchecked_transaction()?;
            let session = tx
                .query_row(
                    &format!("SELECT {SESSION_COLUMNS} FROM sessions WHERE mac = ?1"),
                    params![mac],
                    session_from_row,
                )
                .optional()?;
            if let Some(session) = &session {
                tx.execute("DELETE FROM sessions WHERE mac = ?1", params![mac])?;
                enqueue_session_end(&tx, session, &reason, now)?;
            }
            tx.commit()?;
            Ok(session)
        })
        .await
    }

    pub async fn rebase_untrusted_sessions(&self, now: i64) -> Result<usize> {
        self.with_conn(move |conn| {
            conn.execute(
                "UPDATE sessions
                 SET expires_at = ?1 + (expires_at - granted_at),
                     granted_at = ?1,
                     clock_untrusted = 0
                 WHERE clock_untrusted = 1",
                params![now],
            )
        })
        .await
    }

    pub async fn prune_synced(&self, older_than: i64) -> Result<usize> {
        self.with_conn(move |conn| {
            conn.execute(
                "DELETE FROM sync_queue WHERE synced_at IS NOT NULL AND synced_at < ?1",
                params![older_than],
            )
        })
        .await
    }

    pub async fn cap_pending_events(&self, max_pending: u32) -> Result<usize> {
        self.with_conn(move |conn| {
            conn.execute(
                "DELETE FROM sync_queue
                 WHERE synced_at IS NULL AND id NOT IN (
                    SELECT id FROM sync_queue WHERE synced_at IS NULL
                    ORDER BY id DESC LIMIT ?1
                 )",
                params![max_pending],
            )
        })
        .await
    }

    pub async fn set_meta(&self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let key = key.into();
        let value = value.into();
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn get_meta(&self, key: impl Into<String>) -> Result<Option<String>> {
        let key = key.into();
        self.with_conn(move |conn| {
            conn.query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
        })
        .await
    }

    pub async fn save_policy(&self, policy: &StoredPolicy) -> Result<()> {
        self.set_meta(
            "policy.session_duration_secs",
            policy.session_duration_secs.to_string(),
        )
        .await?;
        self.set_meta(
            "policy.redirect_url",
            policy.redirect_url.clone().unwrap_or_default(),
        )
        .await
    }

    pub async fn load_policy(&self) -> Result<Option<StoredPolicy>> {
        let Some(duration) = self.get_meta("policy.session_duration_secs").await? else {
            return Ok(None);
        };
        let Ok(session_duration_secs) = duration.parse::<u64>() else {
            return Ok(None);
        };
        let redirect_url = self
            .get_meta("policy.redirect_url")
            .await?
            .filter(|url| !url.is_empty());
        Ok(Some(StoredPolicy {
            session_duration_secs,
            redirect_url,
        }))
    }

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
                let kind = event_type.parse::<SyncEventKind>().map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        1,
                        "event_type".to_owned(),
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
