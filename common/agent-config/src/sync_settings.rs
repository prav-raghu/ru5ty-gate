use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SyncSettings {
    #[serde(default = "SyncSettings::default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "SyncSettings::default_batch_size")]
    pub batch_size: u32,
}

impl SyncSettings {
    fn default_interval_secs() -> u64 {
        30
    }

    fn default_batch_size() -> u32 {
        100
    }
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            interval_secs: Self::default_interval_secs(),
            batch_size: Self::default_batch_size(),
        }
    }
}
