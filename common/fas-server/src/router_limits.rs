use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RouterLimits {
    pub request_timeout: Duration,
    pub max_concurrent_requests: usize,
    pub fas_requests_per_minute: u32,
}

impl Default for RouterLimits {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
            max_concurrent_requests: 64,
            fas_requests_per_minute: 30,
        }
    }
}
