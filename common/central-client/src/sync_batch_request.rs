use serde::Serialize;

use crate::SyncEventDto;

#[derive(Debug, Clone, Serialize)]
pub struct SyncBatchRequest {
    pub venue_id: String,
    pub events: Vec<SyncEventDto>,
}
