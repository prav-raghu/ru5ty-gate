use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceVenuePath {
    #[serde(rename = "venueRef")]
    pub venue_ref: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ValidateSessionBody {
    #[validate(length(min = 1, max = 128))]
    pub mac: String,
    #[validate(length(min = 1, max = 256))]
    pub token: String,
    #[validate(length(min = 1, max = 128))]
    pub gateway_name: String,
    #[validate(length(min = 1, max = 128))]
    pub client_ip: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct HeartbeatBody {
    #[validate(length(min = 1, max = 64))]
    pub venue_id: String,
    pub uptime_secs: u64,
    #[validate(range(min = -1))]
    pub active_sessions: i64,
    #[validate(range(min = -1))]
    pub pending_events: i64,
    pub last_sync_ok_at: Option<i64>,
    #[validate(length(min = 1, max = 64))]
    pub agent_version: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SyncEventBody {
    #[validate(length(min = 1, max = 64))]
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: i64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct SyncBatchBody {
    #[validate(length(min = 1, max = 64))]
    pub venue_id: String,
    #[validate(length(max = 1000), nested)]
    pub events: Vec<SyncEventBody>,
}
