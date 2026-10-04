use std::time::Duration;

use ru5ty_gate_session_store::{SessionStore, unix_now};
use tokio::task::JoinHandle;

pub const EXPIRY_SWEEP_INTERVAL: Duration = Duration::from_secs(60);

pub fn spawn_expiry_sweep_task(store: SessionStore, interval: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            match store.delete_expired(unix_now()).await {
                Ok(0) => {}
                Ok(removed) => tracing::debug!(removed, "swept expired sessions"),
                Err(err) => tracing::warn!(?err, "failed to sweep expired sessions"),
            }
        }
    })
}
