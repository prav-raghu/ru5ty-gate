use std::net::SocketAddr;
use std::time::{Duration, Instant};

use ru5ty_gate_agent_config::Settings;
use ru5ty_gate_central_client::{CentralClient, ClientConfig, IdentifierPolicy};
use ru5ty_gate_fas_server::{AppState, FasConfig, RouterLimits, admin_router, public_router};
use ru5ty_gate_heartbeat::{
    HeartbeatTaskConfig, SyncStatus, SyncTaskConfig, spawn_heartbeat_task, spawn_policy_task,
    spawn_sync_task,
};
use ru5ty_gate_ndsctl::Ndsctl;
use ru5ty_gate_session_store::SessionStore;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::jobs::{EXPIRY_SWEEP_INTERVAL, spawn_expiry_sweep_task};
use crate::shutdown::shutdown_signal;
use crate::{AGENT_VERSION, AgentError};

pub struct Application {
    settings: Settings,
    state: AppState,
    ndsctl: Ndsctl,
    started_at: Instant,
}

async fn bind(addr: &str) -> Result<TcpListener, AgentError> {
    TcpListener::bind(addr)
        .await
        .map_err(|source| AgentError::Bind {
            addr: addr.to_owned(),
            source,
        })
}

async fn wait_for_stop(mut stop: watch::Receiver<bool>) {
    let _ = stop.wait_for(|stopped| *stopped).await;
}

async fn supervise(task: &'static str, handle: &mut JoinHandle<()>) -> AgentError {
    if let Err(err) = handle.await {
        tracing::error!(?err, task, "background task failed");
    }
    AgentError::TaskStopped { task }
}

impl Application {
    pub fn initialize(settings: Settings) -> Result<Self, AgentError> {
        let started_at = Instant::now();

        tracing::info!(
            venue_id = %settings.venue.id,
            venue_name = %settings.venue.name,
            bind_addr = %settings.server.bind_addr,
            admin_bind_addr = %settings.server.admin_bind_addr,
            "starting ru5ty-gate agent"
        );

        let store =
            SessionStore::open(&settings.server.db_path).map_err(|source| AgentError::Store {
                path: settings.server.db_path.clone(),
                source,
            })?;

        let central = CentralClient::new(ClientConfig {
            base_url: settings.central.base_url.clone(),
            api_key: settings.central.api_key.clone(),
            timeout: Duration::from_secs(settings.central.timeout_secs),
            venue_id: settings.venue.id.clone(),
        })?;

        let config = FasConfig {
            venue_id: settings.venue.id.clone(),
            gateway_name: settings.venue.gateway_name.clone(),
            default_session_secs: settings.session.default_duration_secs,
            allow_offline: settings.session.allow_offline,
            faskey: settings.fas.faskey.clone(),
            verify_client_ip: settings.fas.verify_client_ip,
        };

        let identifiers = match (
            &settings.privacy.send_raw_identifiers,
            &settings.privacy.hash_pepper,
        ) {
            (false, Some(pepper)) => IdentifierPolicy::hashed(pepper.clone()),
            _ => IdentifierPolicy::raw(),
        };

        let ndsctl = settings
            .enforcement
            .ndsctl_path
            .clone()
            .map_or_else(Ndsctl::disabled, Ndsctl::with_program);

        Ok(Self {
            state: AppState {
                store,
                central,
                config,
                identifiers,
            },
            ndsctl,
            settings,
            started_at,
        })
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub async fn start(self) -> Result<(), AgentError> {
        let Self {
            settings,
            state,
            ndsctl,
            started_at,
        } = self;

        let public_listener = bind(&settings.server.bind_addr).await?;
        let admin_listener = bind(&settings.server.admin_bind_addr).await?;

        let status = SyncStatus::new();
        let report_interval = Duration::from_secs(settings.heartbeat.interval_secs);
        let mut heartbeat = spawn_heartbeat_task(
            state.store.clone(),
            state.central.clone(),
            status.clone(),
            HeartbeatTaskConfig {
                venue_id: settings.venue.id.clone(),
                agent_version: AGENT_VERSION.to_owned(),
                interval: report_interval,
                process_start: started_at,
            },
        );
        let mut sync = spawn_sync_task(
            state.store.clone(),
            state.central.clone(),
            state.identifiers.clone(),
            status,
            SyncTaskConfig {
                interval: Duration::from_secs(settings.sync.interval_secs),
                batch_size: settings.sync.batch_size,
                max_pending: settings.sync.max_pending,
                retain_synced_secs: settings.sync.retain_synced_secs,
            },
        );
        let mut policy =
            spawn_policy_task(state.store.clone(), state.central.clone(), report_interval);
        let mut sweep = spawn_expiry_sweep_task(state.store.clone(), ndsctl, EXPIRY_SWEEP_INTERVAL);

        let limits = RouterLimits {
            request_timeout: Duration::from_secs(settings.limits.request_timeout_secs),
            max_concurrent_requests: settings.limits.max_concurrent_requests,
            fas_requests_per_minute: settings.limits.fas_requests_per_minute,
        };
        let (stop_sender, stop) = watch::channel(false);
        tokio::spawn(async move {
            shutdown_signal().await;
            let _ = stop_sender.send(true);
        });

        tracing::info!(addr = %settings.server.bind_addr, "FAS server listening");
        tracing::info!(addr = %settings.server.admin_bind_addr, "admin server listening");
        let public = axum::serve(
            public_listener,
            public_router(state.clone(), limits)
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(wait_for_stop(stop.clone()));
        let admin = axum::serve(admin_listener, admin_router(state))
            .with_graceful_shutdown(wait_for_stop(stop));

        let outcome = tokio::select! {
            served = async { tokio::try_join!(public.into_future(), admin.into_future()) } => {
                served.map(drop).map_err(AgentError::Serve)
            }
            error = supervise("heartbeat", &mut heartbeat) => Err(error),
            error = supervise("sync", &mut sync) => Err(error),
            error = supervise("policy", &mut policy) => Err(error),
            error = supervise("expiry sweep", &mut sweep) => Err(error),
        };

        tracing::info!("shutting down, stopping background tasks");
        heartbeat.abort();
        sync.abort();
        policy.abort();
        sweep.abort();

        outcome
    }
}
