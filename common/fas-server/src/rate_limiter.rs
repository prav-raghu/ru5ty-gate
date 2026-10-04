use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_TRACKED_CLIENTS: usize = 4096;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    windows: Arc<Mutex<HashMap<IpAddr, (Instant, u32)>>>,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            windows: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn per_minute(limit: u32) -> Self {
        Self::new(limit, Duration::from_secs(60))
    }

    pub fn check(&self, client: IpAddr) -> bool {
        let now = Instant::now();
        let Ok(mut windows) = self.windows.lock() else {
            return false;
        };
        if windows.len() >= MAX_TRACKED_CLIENTS {
            windows.retain(|_, (started, _)| now.duration_since(*started) < self.window);
        }
        let entry = windows.entry(client).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        if entry.1 >= self.limit {
            return false;
        }
        entry.1 += 1;
        true
    }
}
