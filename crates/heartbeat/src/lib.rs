//! Scheduled background tasks: periodic device heartbeat, and batched sync
//! of buffered session events once the central platform is reachable
//! again.
//!
//! Both tasks are deliberately independent of each other and of the FAS
//! request path -- a slow or unreachable central platform here never
//! blocks a client's captive-portal auth.

use std::time::{Duration, Instant};

use central_client::{CentralClient, HeartbeatRequest, SyncEventDto};
use session_store::SessionStore;
use tokio::task::JoinHandle;

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Spawn the heartbeat task: every `interval`, post uptime + active
/// session count to the central platform. A failed post is logged and
/// retried on the next tick -- there's no backoff or buffering here, since
/// heartbeats are inherently a "latest state" signal, not an event log.
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
        // First tick fires immediately; skip it so we don't heartbeat
        // before the server has even finished starting up.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = now_unix();
            let uptime_secs = process_start.elapsed().as_secs();
            let active_sessions = match store.active_session_count(now).await {
                Ok(count) => count,
                Err(err) => {
                    tracing::warn!(?err, "failed to read active session count for heartbeat");
                    -1
                }
            };

            let hb = HeartbeatRequest {
                venue_id: venue_id.clone(),
                uptime_secs,
                active_sessions,
                agent_version: agent_version.clone(),
                timestamp: now,
            };

            match central.post_heartbeat(&hb).await {
                Ok(()) => tracing::debug!(uptime_secs, active_sessions, "heartbeat sent"),
                Err(err) => {
                    tracing::warn!(?err, "heartbeat failed, will retry next interval")
                }
            }
        }
    })
}

/// Spawn the sync task: every `interval`, pull up to `batch_size` buffered
/// session events and push them to the central platform. Events stay
/// queued (and get retried) until the central platform actually
/// acknowledges them -- this is what makes the local buffer survive a
/// central-platform outage without losing session history.
pub fn spawn_sync_task(
    store: SessionStore,
    central: CentralClient,
    interval: Duration,
    batch_size: u32,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;

            let pending = match store.pending_events(batch_size).await {
                Ok(events) => events,
                Err(err) => {
                    tracing::warn!(?err, "failed to read pending sync events");
                    continue;
                }
            };
            if pending.is_empty() {
                continue;
            }

            let ids: Vec<i64> = pending.iter().map(|e| e.id).collect();
            let dtos: Vec<SyncEventDto> = pending
                .into_iter()
                .map(|e| SyncEventDto {
                    event_type: match e.kind {
                        session_store::SyncEventKind::SessionStart => "session_start".to_string(),
                        session_store::SyncEventKind::SessionEnd => "session_end".to_string(),
                    },
                    payload: e.payload,
                    occurred_at: e.created_at,
                })
                .collect();

            let batch_len = dtos.len();
            match central.sync_events(dtos).await {
                Ok(_) => {
                    if let Err(err) = store.mark_synced(ids, now_unix()).await {
                        tracing::error!(
                            ?err,
                            "synced events but failed to mark them synced locally"
                        );
                    } else {
                        tracing::info!(count = batch_len, "synced buffered session events");
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        ?err,
                        count = batch_len,
                        "sync failed, will retry next interval"
                    );
                }
            }
        }
    })
}
