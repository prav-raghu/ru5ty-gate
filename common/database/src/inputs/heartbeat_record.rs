use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct HeartbeatRecord {
    pub agent_version: String,
    pub uptime_secs: i64,
    pub active_sessions: i32,
    pub pending_events: i32,
    pub last_sync_ok_at: Option<DateTime<Utc>>,
}
