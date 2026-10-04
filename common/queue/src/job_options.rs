use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl JobPriority {
    pub fn value(self) -> u8 {
        match self {
            Self::Critical => 1,
            Self::High => 2,
            Self::Normal => 3,
            Self::Low => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackoffKind {
    Fixed,
    Exponential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackoffOptions {
    pub kind: BackoffKind,
    pub delay_ms: u64,
}

impl BackoffOptions {
    pub fn delay_for_attempt(self, attempt: u32) -> u64 {
        match self.kind {
            BackoffKind::Fixed => self.delay_ms,
            BackoffKind::Exponential => self
                .delay_ms
                .saturating_mul(1u64 << attempt.saturating_sub(1).min(20)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueueJobOptions {
    pub priority: Option<JobPriority>,
    pub delay_ms: Option<u64>,
    pub attempts: Option<u32>,
    pub backoff: Option<BackoffOptions>,
    pub job_id: Option<String>,
}
