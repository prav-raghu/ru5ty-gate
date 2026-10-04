use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct SyncTaskConfig {
    pub interval: Duration,
    pub batch_size: u32,
    pub max_pending: u32,
    pub retain_synced_secs: u64,
}
