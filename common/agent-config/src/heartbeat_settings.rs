use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatSettings {
    #[serde(default = "HeartbeatSettings::default_interval_secs")]
    pub interval_secs: u64,
}

impl HeartbeatSettings {
    fn default_interval_secs() -> u64 {
        60
    }
}

impl Default for HeartbeatSettings {
    fn default() -> Self {
        Self {
            interval_secs: Self::default_interval_secs(),
        }
    }
}
