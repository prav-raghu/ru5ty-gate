use ru5ty_gate_central_client::{CentralClient, HeartbeatRequest};
use ru5ty_gate_session_store::{SessionStore, unix_now};
use tokio::task::JoinHandle;

use crate::{HeartbeatTaskConfig, SyncStatus};

pub fn spawn_heartbeat_task(
    store: SessionStore,
    central: CentralClient,
    status: SyncStatus,
    config: HeartbeatTaskConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(config.interval);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = unix_now();
            let uptime_secs = config.process_start.elapsed().as_secs();
            let active_sessions = match store.active_session_count(now).await {
                Ok(count) => count,
                Err(err) => {
                    tracing::warn!(?err, "failed to read active session count for heartbeat");
                    -1
                }
            };
            let pending_events = store.pending_event_count().await.unwrap_or(-1);

            let heartbeat = HeartbeatRequest {
                venue_id: config.venue_id.clone(),
                uptime_secs,
                active_sessions,
                pending_events,
                last_sync_ok_at: status.last_ok(),
                agent_version: config.agent_version.clone(),
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
