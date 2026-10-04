use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

const NEVER: i64 = i64::MIN;

#[derive(Debug, Clone)]
pub struct SyncStatus {
    last_ok: Arc<AtomicI64>,
}

impl SyncStatus {
    pub fn new() -> Self {
        Self {
            last_ok: Arc::new(AtomicI64::new(NEVER)),
        }
    }

    pub fn mark_ok(&self, at: i64) {
        self.last_ok.store(at, Ordering::Relaxed);
    }

    pub fn last_ok(&self) -> Option<i64> {
        match self.last_ok.load(Ordering::Relaxed) {
            NEVER => None,
            at => Some(at),
        }
    }
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self::new()
    }
}
