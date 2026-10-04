use rusqlite::Connection;

pub const CURRENT_SCHEMA_VERSION: i64 = 2;

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        mac              TEXT PRIMARY KEY,
        token            TEXT NOT NULL,
        venue            TEXT NOT NULL,
        granted_at       INTEGER NOT NULL,
        expires_at       INTEGER NOT NULL,
        redirect_url     TEXT,
        clock_untrusted  INTEGER NOT NULL DEFAULT 0
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

    CREATE INDEX IF NOT EXISTS idx_sync_queue_synced_at
        ON sync_queue (synced_at) WHERE synced_at IS NOT NULL;

    CREATE TABLE IF NOT EXISTS meta (
        key    TEXT PRIMARY KEY,
        value  TEXT NOT NULL
    );
";

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(SCHEMA)?;
    if !column_exists(conn, "sessions", "clock_untrusted")? {
        conn.execute_batch(
            "ALTER TABLE sessions ADD COLUMN clock_untrusted INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}
