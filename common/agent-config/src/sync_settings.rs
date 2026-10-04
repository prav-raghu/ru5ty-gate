use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SyncSettings {
    #[serde(default = "SyncSettings::default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "SyncSettings::default_batch_size")]
    pub batch_size: u32,
    #[serde(default = "SyncSettings::default_retain_synced_secs")]
    pub retain_synced_secs: u64,
    #[serde(default = "SyncSettings::default_max_pending")]
    pub max_pending: u32,
}

impl SyncSettings {
    fn default_interval_secs() -> u64 {
        30
    }

    fn default_batch_size() -> u32 {
        100
    }

    fn default_retain_synced_secs() -> u64 {
        86_400
    }

    fn default_max_pending() -> u32 {
        50_000
    }
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            interval_secs: Self::default_interval_secs(),
            batch_size: Self::default_batch_size(),
            retain_synced_secs: Self::default_retain_synced_secs(),
            max_pending: Self::default_max_pending(),
        }
    }
}
