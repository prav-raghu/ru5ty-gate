use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(3);

async fn probe(addr: &str) -> std::io::Result<bool> {
    let mut stream = TcpStream::connect(addr).await?;
    stream
        .write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .await?;
    let mut buffer = [0_u8; 64];
    let read = stream.read(&mut buffer).await?;
    let response = &buffer[..read];
    Ok(response.starts_with(b"HTTP/1.0 200") || response.starts_with(b"HTTP/1.1 200"))
}

pub async fn run_healthcheck(bind_addr: &str) -> bool {
    let target = match bind_addr.rsplit_once(':') {
        Some(("0.0.0.0" | "[::]", port)) => format!("127.0.0.1:{port}"),
        _ => bind_addr.to_owned(),
    };
    matches!(
        tokio::time::timeout(HEALTHCHECK_TIMEOUT, probe(&target)).await,
        Ok(Ok(true))
    )
}
