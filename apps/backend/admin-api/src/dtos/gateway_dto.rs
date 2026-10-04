use chrono::{DateTime, Utc};
use ru5ty_gate_database::Gateway;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayDto {
    pub id: Uuid,
    pub venue_id: Uuid,
    pub name: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub agent_version: Option<String>,
    pub uptime_secs: Option<i64>,
    pub active_sessions: Option<i32>,
    pub pending_events: Option<i32>,
    pub last_sync_ok_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Gateway> for GatewayDto {
    fn from(gateway: Gateway) -> Self {
        Self {
            id: gateway.id,
            venue_id: gateway.venue_id,
            name: gateway.name,
            last_heartbeat_at: gateway.last_heartbeat_at,
            agent_version: gateway.agent_version,
            uptime_secs: gateway.uptime_secs,
            active_sessions: gateway.active_sessions,
            pending_events: gateway.pending_events,
            last_sync_ok_at: gateway.last_sync_ok_at,
            created_at: gateway.created_at,
        }
    }
}
