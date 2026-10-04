#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ru5ty_gate_agent::Application;
use ru5ty_gate_agent_config::Settings;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

pub const FASKEY: &str = "0123456789abcdef0123456789abcdef";

pub fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ru5ty-gate-agent-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub async fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

pub struct Ports {
    pub public: u16,
    pub admin: u16,
    pub central: u16,
}

impl Ports {
    pub async fn allocate() -> Self {
        Self {
            public: free_port().await,
            admin: free_port().await,
            central: free_port().await,
        }
    }
}

pub fn config_toml(ports: &Ports, db_path: &str, extra: &str) -> String {
    format!(
        r#"
        [venue]
        id = "venue-1"
        gateway_name = "gw1"

        [server]
        bind_addr = "127.0.0.1:{public}"
        admin_bind_addr = "127.0.0.1:{admin}"
        db_path = "{db_path}"

        [fas]
        faskey = "{FASKEY}"

        [central]
        base_url = "http://127.0.0.1:{central}"
        timeout_secs = 1

        [heartbeat]
        interval_secs = 1

        [sync]
        interval_secs = 1

        [privacy]
        send_raw_identifiers = true
        {extra}
    "#,
        public = ports.public,
        admin = ports.admin,
        central = ports.central,
    )
}

pub fn settings(ports: &Ports, db_path: &str, extra: &str) -> Settings {
    Settings::parse(&config_toml(ports, db_path, extra), "inline").unwrap()
}

pub struct RunningAgent {
    pub ports: Ports,
    pub dir: PathBuf,
    pub handle: JoinHandle<()>,
}

impl RunningAgent {
    pub async fn start(name: &str, ports: Ports, extra: &str) -> Self {
        let dir = scratch_dir(name);
        let db_path = dir.join("sessions.db");
        let settings = settings(&ports, db_path.to_str().unwrap(), extra);
        let application = Application::initialize(settings).unwrap();
        let handle = tokio::spawn(async move {
            let _ = application.start().await;
        });
        let agent = Self { ports, dir, handle };
        agent.wait_until_listening().await;
        agent
    }

    async fn wait_until_listening(&self) {
        for _ in 0..100 {
            if TcpStream::connect(self.public_addr()).await.is_ok()
                && TcpStream::connect(self.admin_addr()).await.is_ok()
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("agent did not start listening");
    }

    pub fn public_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.ports.public))
    }

    pub fn admin_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.ports.admin))
    }

    pub fn stop(self) {
        self.handle.abort();
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

pub struct HttpResponse {
    pub status: u16,
    pub location: Option<String>,
    pub body: String,
}

pub async fn http(addr: SocketAddr, method: &str, path: &str, body: &str) -> HttpResponse {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.0\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap();
    let location = head.lines().find_map(|line| {
        line.strip_prefix("location: ")
            .or_else(|| line.strip_prefix("Location: "))
            .map(str::to_owned)
    });
    HttpResponse {
        status,
        location,
        body: body.to_owned(),
    }
}

pub fn fas_path(mac: &str, hid: &str) -> String {
    let payload = format!(
        "clientip=127.0.0.1, clientmac={mac}, gatewayname=gw1, hid={hid}, \
         gatewayaddress=10.0.0.1:2050, authdir=opennds_auth, originurl=http%3A%2F%2Fexample.com"
    );
    let encoded = STANDARD
        .encode(payload)
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D");
    format!("/fas?fas={encoded}")
}
