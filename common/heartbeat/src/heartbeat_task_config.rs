use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct HeartbeatTaskConfig {
    pub venue_id: String,
    pub agent_version: String,
    pub interval: Duration,
    pub process_start: Instant,
}
