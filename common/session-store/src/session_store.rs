use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{Result, Session, StoreError, SyncEvent, SyncEventKind, schema};

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

    pub async fn delete_expired(&self, now: i64) -> Result<usize> {
        self.with_conn(move |conn| {
            conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now])
        })
        .await
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
