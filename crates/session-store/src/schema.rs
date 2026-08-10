//! Sqlite schema + migrations for the local session store.

use rusqlite::Connection;

/// Bumped whenever `SCHEMA` changes in a way that needs a migration step
/// beyond `CREATE TABLE IF NOT EXISTS`. v1 has no migrations yet -- this
/// exists so a future breaking schema change has somewhere to hook in.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        mac           TEXT PRIMARY KEY,
        token         TEXT NOT NULL,
        venue         TEXT NOT NULL,
        granted_at    INTEGER NOT NULL,
        expires_at    INTEGER NOT NULL,
        redirect_url  TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions (expires_at);

    CREATE TABLE IF NOT EXISTS sync_queue (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        event_type  TEXT NOT NULL,
        payload     TEXT NOT NULL,
        created_at  INTEGER NOT NULL,
        synced_at   INTEGER
    );

    CREATE INDEX IF NOT EXISTS idx_sync_queue_pending
        ON sync_queue (id) WHERE synced_at IS NULL;
";

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(SCHEMA)?;
    conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}
