use std::time::{Duration, Instant};

use ru5ty_gate_central_client::{CentralClient, HeartbeatRequest};
use ru5ty_gate_session_store::{SessionStore, unix_now};
use tokio::task::JoinHandle;

pub fn spawn_heartbeat_task(
    store: SessionStore,
    central: CentralClient,
    venue_id: String,
    agent_version: String,
    interval: Duration,
    process_start: Instant,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = unix_now();
            let uptime_secs = process_start.elapsed().as_secs();
            let active_sessions = match store.active_session_count(now).await {
                Ok(count) => count,
                Err(err) => {
                    tracing::warn!(?err, "failed to read active session count for heartbeat");
                    -1
                }
            };

            let heartbeat = HeartbeatRequest {
                venue_id: venue_id.clone(),
                uptime_secs,
                active_sessions,
                agent_version: agent_version.clone(),
                timestamp: now,
            };

            match central.post_heartbeat(&heartbeat).await {
                Ok(()) => tracing::debug!(uptime_secs, active_sessions, "heartbeat sent"),
                Err(err) => {
                    tracing::warn!(?err, "heartbeat failed, will retry next interval");
                }
            }
        }
    })
}
