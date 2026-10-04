use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SyncBatchResponse {
    #[serde(default)]
    pub accepted: usize,
}
