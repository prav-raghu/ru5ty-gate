use std::time::Duration;

use ru5ty_gate_ndsctl::Ndsctl;
use ru5ty_gate_session_store::{SessionStore, clock_is_trusted, unix_now};
use tokio::task::JoinHandle;

pub const EXPIRY_SWEEP_INTERVAL: Duration = Duration::from_secs(60);

pub async fn sweep_once(store: &SessionStore, ndsctl: &Ndsctl, now: i64) -> usize {
    if !clock_is_trusted(now) {
        tracing::warn!("system clock is not trusted yet, skipping the expiry sweep");
        return 0;
    }
    match store.rebase_untrusted_sessions(now).await {
        Ok(0) => {}
        Ok(rebased) => tracing::info!(
            rebased,
            "restarted sessions granted before the clock was set"
        ),
        Err(err) => tracing::warn!(
            ?err,
            "failed to rebase sessions granted under an untrusted clock"
        ),
    }
    let expired = match store.sweep_expired(now).await {
        Ok(expired) => expired,
        Err(err) => {
            tracing::warn!(?err, "failed to sweep expired sessions");
            return 0;
        }
    };
    for session in &expired {
        if let Err(err) = ndsctl.deauth(&session.mac).await {
            tracing::warn!(
                ?err,
                "failed to deauthenticate an expired client in openNDS"
            );
        }
    }
    if !expired.is_empty() {
        tracing::debug!(removed = expired.len(), "swept expired sessions");
    }
    expired.len()
}

pub fn spawn_expiry_sweep_task(
    store: SessionStore,
    ndsctl: Ndsctl,
    interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            sweep_once(&store, &ndsctl, unix_now()).await;
        }
    })
}
