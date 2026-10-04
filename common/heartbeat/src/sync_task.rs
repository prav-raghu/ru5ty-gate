use std::time::Duration;

use ru5ty_gate_central_client::{CentralClient, SyncEventDto};
use ru5ty_gate_session_store::{SessionStore, unix_now};
use tokio::task::JoinHandle;

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

            let ids: Vec<i64> = pending.iter().map(|event| event.id).collect();
            let dtos: Vec<SyncEventDto> = pending
                .into_iter()
                .map(|event| SyncEventDto {
                    event_type: event.kind.as_str().to_owned(),
                    payload: event.payload,
                    occurred_at: event.created_at,
                })
                .collect();

            let batch_len = dtos.len();
            match central.sync_events(dtos).await {
                Ok(_) => {
                    if let Err(err) = store.mark_synced(ids, unix_now()).await {
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
