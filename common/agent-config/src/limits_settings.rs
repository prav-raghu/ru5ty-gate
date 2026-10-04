use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LimitsSettings {
    #[serde(default = "LimitsSettings::default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default = "LimitsSettings::default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
    #[serde(default = "LimitsSettings::default_fas_requests_per_minute")]
    pub fas_requests_per_minute: u32,
}

impl LimitsSettings {
    fn default_request_timeout_secs() -> u64 {
        10
    }

    fn default_max_concurrent_requests() -> usize {
        64
    }

    fn default_fas_requests_per_minute() -> u32 {
        30
    }
}

impl Default for LimitsSettings {
    fn default() -> Self {
        Self {
            request_timeout_secs: Self::default_request_timeout_secs(),
            max_concurrent_requests: Self::default_max_concurrent_requests(),
            fas_requests_per_minute: Self::default_fas_requests_per_minute(),
        }
    }
}
