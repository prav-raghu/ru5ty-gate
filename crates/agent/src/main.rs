//! ru5ty-gate captive portal agent.
//!
//! Router-side daemon that sits behind openNDS as its Forwarding
//! Authentication Service: serves the FAS HTTP endpoint openNDS calls to
//! authenticate clients, persists sessions locally so it survives
//! restarts and central-platform outages, and runs the heartbeat + sync
//! background tasks. See the workspace README for the full protocol
//! surface.

use std::time::{Duration, Instant};

use anyhow::Context;
use clap::Parser;
use tokio::net::TcpListener;

use agent_config::Settings;
use central_client::{CentralClient, ClientConfig};
use fas_server::{AppState, FasConfig};
use session_store::SessionStore;

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "ru5ty-gate-agent", version = AGENT_VERSION)]
struct Cli {
    /// Path to the agent's TOML config file. Falls back to
    /// `$RU5TY_GATE_CONFIG`, then `config/agent.toml`.
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let process_start = Instant::now();
    let cli = Cli::parse();

    let settings = match cli.config {
        Some(path) => Settings::load_from(&path)
            .with_context(|| format!("failed to load config from {path}"))?,
        None => Settings::load().context("failed to load config")?,
    };

    tracing::info!(
        venue_id = %settings.venue.id,
        venue_name = %settings.venue.name,
        bind_addr = %settings.server.bind_addr,
        "starting ru5ty-gate agent"
    );

    let store = SessionStore::open(&settings.server.db_path).with_context(|| {
        format!(
            "failed to open session store at {}",
            settings.server.db_path
        )
    })?;

    let central = CentralClient::new(ClientConfig {
        base_url: settings.central.base_url.clone(),
        api_key: settings.central.api_key.clone(),
        timeout: Duration::from_secs(settings.central.timeout_secs),
        venue_id: settings.venue.id.clone(),
    })
    .context("failed to build central platform client")?;

    let fas_config = FasConfig {
        venue_id: settings.venue.id.clone(),
        gateway_name: settings.venue.gateway_name.clone(),
        default_session_secs: settings.session.default_duration_secs,
        allow_offline: settings.session.allow_offline,
    };

    let app_state = AppState {
        store: store.clone(),
        central: central.clone(),
        config: fas_config,
    };

    // Background tasks: heartbeat check-in and buffered-event sync run
    // independently of the request path so an unreachable central
    // platform never blocks a client's captive-portal auth.
    let heartbeat_handle = heartbeat::spawn_heartbeat_task(
        store.clone(),
        central.clone(),
        settings.venue.id.clone(),
        AGENT_VERSION.to_string(),
        Duration::from_secs(settings.heartbeat.interval_secs),
        process_start,
    );
    let sync_handle = heartbeat::spawn_sync_task(
        store.clone(),
        central.clone(),
        Duration::from_secs(settings.sync.interval_secs),
        settings.sync.batch_size,
    );
    let sweep_handle = spawn_expiry_sweep_task(store.clone(), Duration::from_secs(60));

    let listener = TcpListener::bind(&settings.server.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", settings.server.bind_addr))?;
    tracing::info!(addr = %settings.server.bind_addr, "FAS server listening");

    let router = fas_server::router(app_state);
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("FAS server error")?;

    tracing::info!("shutting down, stopping background tasks");
    heartbeat_handle.abort();
    sync_handle.abort();
    sweep_handle.abort();

    Ok(())
}

/// Periodically sweep expired sessions out of the local store so it
/// doesn't grow unbounded on a device that stays up for a long time.
fn spawn_expiry_sweep_task(store: SessionStore, interval: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let now = chrono::Utc::now().timestamp();
            match store.delete_expired(now).await {
                Ok(0) => {}
                Ok(n) => tracing::debug!(removed = n, "swept expired sessions"),
                Err(err) => tracing::warn!(?err, "failed to sweep expired sessions"),
            }
        }
    })
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
