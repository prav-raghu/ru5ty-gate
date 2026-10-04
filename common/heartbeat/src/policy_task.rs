use std::time::Duration;

use ru5ty_gate_central_client::CentralClient;
use ru5ty_gate_session_store::{SessionStore, StoredPolicy};
use tokio::task::JoinHandle;

pub fn spawn_policy_task(
    store: SessionStore,
    central: CentralClient,
    interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            match central.fetch_policy().await {
                Ok(policy) => {
                    let stored = StoredPolicy {
                        session_duration_secs: policy.session_duration_secs,
                        redirect_url: policy.redirect_url,
                    };
                    if let Err(err) = store.save_policy(&stored).await {
                        tracing::warn!(?err, "failed to cache central policy");
                    }
                }
                Err(err) => tracing::warn!(?err, "failed to fetch central policy"),
            }
        }
    })
}
