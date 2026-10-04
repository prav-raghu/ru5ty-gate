use std::time::{Duration, Instant};

use ru5ty_gate_agent_config::Settings;
use ru5ty_gate_central_client::{CentralClient, ClientConfig};
use ru5ty_gate_fas_server::{AppState, FasConfig, router};
use ru5ty_gate_heartbeat::{spawn_heartbeat_task, spawn_sync_task};
use ru5ty_gate_session_store::SessionStore;
use tokio::net::TcpListener;

use crate::jobs::{EXPIRY_SWEEP_INTERVAL, spawn_expiry_sweep_task};
use crate::shutdown::shutdown_signal;
use crate::{AGENT_VERSION, AgentError};

pub struct Application {
    settings: Settings,
    state: AppState,
    started_at: Instant,
}

impl Application {
    pub fn initialize(settings: Settings) -> Result<Self, AgentError> {
        let started_at = Instant::now();

        tracing::info!(
            venue_id = %settings.venue.id,
            venue_name = %settings.venue.name,
            bind_addr = %settings.server.bind_addr,
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
        };

        Ok(Self {
            settings,
            state: AppState {
                store,
                central,
                config,
            },
            started_at,
        })
    }

    pub async fn start(self) -> Result<(), AgentError> {
        let Self {
            settings,
            state,
            started_at,
        } = self;

        let heartbeat_handle = spawn_heartbeat_task(
            state.store.clone(),
            state.central.clone(),
            settings.venue.id.clone(),
            AGENT_VERSION.to_owned(),
            Duration::from_secs(settings.heartbeat.interval_secs),
            started_at,
        );
        let sync_handle = spawn_sync_task(
            state.store.clone(),
            state.central.clone(),
            Duration::from_secs(settings.sync.interval_secs),
            settings.sync.batch_size,
        );
        let sweep_handle = spawn_expiry_sweep_task(state.store.clone(), EXPIRY_SWEEP_INTERVAL);

        let listener = TcpListener::bind(&settings.server.bind_addr)
            .await
            .map_err(|source| AgentError::Bind {
                addr: settings.server.bind_addr.clone(),
                source,
            })?;
        tracing::info!(addr = %settings.server.bind_addr, "FAS server listening");

        let served = axum::serve(listener, router(state))
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(AgentError::Serve);

        tracing::info!("shutting down, stopping background tasks");
        heartbeat_handle.abort();
        sync_handle.abort();
        sweep_handle.abort();

        served
    }
}
