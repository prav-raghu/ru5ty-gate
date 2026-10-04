use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SyncEventDto {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: i64,
}
