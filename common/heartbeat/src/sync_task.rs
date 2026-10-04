use ru5ty_gate_central_client::{CentralClient, IdentifierPolicy, SyncEventDto};
use ru5ty_gate_session_store::{SessionStore, unix_now};
use tokio::task::JoinHandle;

use crate::{SyncStatus, SyncTaskConfig};

async fn run_maintenance(store: &SessionStore, config: &SyncTaskConfig) {
    match store.cap_pending_events(config.max_pending).await {
        Ok(0) => {}
        Ok(dropped) => tracing::warn!(dropped, "sync queue is full, dropped the oldest events"),
        Err(err) => tracing::warn!(?err, "failed to cap the sync queue"),
    }
    let retain = i64::try_from(config.retain_synced_secs).unwrap_or(i64::MAX);
    match store.prune_synced(unix_now().saturating_sub(retain)).await {
        Ok(0) => {}
        Ok(pruned) => tracing::debug!(pruned, "pruned synced events"),
        Err(err) => tracing::warn!(?err, "failed to prune synced events"),
    }
}

pub fn spawn_sync_task(
    store: SessionStore,
    central: CentralClient,
    identifiers: IdentifierPolicy,
    status: SyncStatus,
    config: SyncTaskConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(config.interval);
        loop {
            ticker.tick().await;
            run_maintenance(&store, &config).await;

            let pending = match store.pending_events(config.batch_size).await {
                Ok(events) => events,
                Err(err) => {
                    tracing::warn!(?err, "failed to read pending sync events");
                    continue;
                }
            };
            if pending.is_empty() {
                continue;
            }

            let ids: Vec<i64> = pending.iter().map(|event| event.id).collect();
            let dtos: Vec<SyncEventDto> = pending
                .into_iter()
                .map(|event| SyncEventDto {
                    event_type: event.kind.as_str().to_owned(),
                    payload: identifiers.apply_to_payload(event.payload),
                    occurred_at: event.created_at,
                })
                .collect();

            let batch_len = dtos.len();
            match central.sync_events(dtos).await {
                Ok(_) => {
                    let now = unix_now();
                    if let Err(err) = store.mark_synced(ids, now).await {
                        tracing::error!(
                            ?err,
                            "synced events but failed to mark them synced locally"
                        );
                    } else {
                        status.mark_ok(now);
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
