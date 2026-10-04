use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SessionSettings {
    #[serde(default = "SessionSettings::default_duration_secs")]
    pub default_duration_secs: u64,
    #[serde(default = "SessionSettings::default_allow_offline")]
    pub allow_offline: bool,
}

impl SessionSettings {
    fn default_duration_secs() -> u64 {
        3600
    }

    fn default_allow_offline() -> bool {
        true
    }
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            default_duration_secs: Self::default_duration_secs(),
            allow_offline: Self::default_allow_offline(),
        }
    }
}
