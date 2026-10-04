use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct QueueMetrics {
    pub waiting: u64,
    pub delayed: u64,
    pub completed: u64,
    pub failed: u64,
}
