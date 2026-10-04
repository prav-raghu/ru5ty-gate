use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;

use crate::server_info::ServerInfo;

pub async fn shutdown_signal() {
    let interrupt = async {
        let _ = signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut stream) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            stream.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
}

fn log_startup(info: &ServerInfo) {
    if info.production {
        tracing::info!(
            service = info.service,
            version = info.version,
            environment = info.environment(),
            port = info.port,
            "server started"
        );
        return;
    }
    tracing::info!(
        "{} v{} running on {} ({})",
        info.service,
        info.version,
        info.local_url(),
        info.environment()
    );
    if let Some(docs_url) = info.docs_url() {
        tracing::info!("API docs: {docs_url}");
    }
}

pub async fn serve(router: Router, info: &ServerInfo) -> std::io::Result<()> {
    let address = SocketAddr::from(([0, 0, 0, 0], info.port));
    let listener = TcpListener::bind(address).await?;
    log_startup(info);
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
}

pub fn run_healthcheck(port: u16, path: &str) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let timeout = Duration::from_secs(5);
    let Ok(mut stream) = TcpStream::connect_timeout(&address, timeout) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut buffer = [0_u8; 32];
    let Ok(read) = stream.read(&mut buffer) else {
        return false;
    };
    let head = String::from_utf8_lossy(&buffer[..read]);
    head.starts_with("HTTP/1.1 2") || head.starts_with("HTTP/1.0 2")
}
