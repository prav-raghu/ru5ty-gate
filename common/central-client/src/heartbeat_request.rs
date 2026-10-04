use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatRequest {
    pub venue_id: String,
    pub uptime_secs: u64,
    pub active_sessions: i64,
    pub pending_events: i64,
    pub last_sync_ok_at: Option<i64>,
    pub agent_version: String,
    pub timestamp: i64,
}
