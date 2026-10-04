use crate::SyncEventKind;

#[derive(Debug, Clone)]
pub struct SyncEvent {
    pub id: i64,
    pub kind: SyncEventKind,
    pub payload: serde_json::Value,
    pub created_at: i64,
}
