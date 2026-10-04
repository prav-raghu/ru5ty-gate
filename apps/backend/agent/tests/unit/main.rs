#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../common/mod.rs"]
mod common;

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use clap::Parser;
use common::{Ports, scratch_dir, settings};
use ru5ty_gate_agent::{AgentError, Application, Cli, Command, run_healthcheck, sweep_once};
use ru5ty_gate_ndsctl::Ndsctl;
use ru5ty_gate_session_store::{Session, SessionStore, build_unix};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn cli_loads_settings_from_the_config_flag() {
    let dir = scratch_dir("cli");
    let path = dir.join("agent.toml");
    let ports = Ports::allocate().await;
    std::fs::write(&path, common::config_toml(&ports, "x.db", "")).unwrap();

    let cli = Cli::parse_from(["ru5ty-gate-agent", "--config", path.to_str().unwrap()]);
    let loaded = cli.load_settings().unwrap();

    assert_eq!(loaded.venue.id, "venue-1");
    assert!(cli.command.is_none());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cli_reports_an_error_when_the_config_file_is_missing() {
    let cli = Cli::parse_from(["ru5ty-gate-agent", "--config", "/nonexistent/agent.toml"]);

    assert!(cli.load_settings().is_err());
}

#[test]
fn cli_parses_the_healthcheck_subcommand() {
    let cli = Cli::parse_from(["ru5ty-gate-agent", "healthcheck"]);

    assert!(matches!(cli.command, Some(Command::Healthcheck)));
}

#[tokio::test]
async fn initialize_opens_the_session_store_at_the_configured_path() {
    let dir = scratch_dir("init");
    let db_path = dir.join("sessions.db");
    let ports = Ports::allocate().await;

    let result = Application::initialize(settings(&ports, db_path.to_str().unwrap(), ""));

    assert!(result.is_ok());
    assert!(db_path.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn initialize_fails_when_the_session_store_cannot_be_opened() {
    let dir = scratch_dir("blocked");
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, "not a directory").unwrap();
    let db_path = blocker.join("sessions.db");
    let ports = Ports::allocate().await;

    let result = Application::initialize(settings(&ports, db_path.to_str().unwrap(), ""));

    assert!(matches!(result, Err(AgentError::Store { .. })));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn start_fails_when_a_listen_address_is_already_taken() {
    let dir = scratch_dir("taken");
    let ports = Ports::allocate().await;
    let _occupied = TcpListener::bind(("127.0.0.1", ports.public))
        .await
        .unwrap();
    let application = Application::initialize(settings(
        &ports,
        dir.join("sessions.db").to_str().unwrap(),
        "",
    ))
    .unwrap();

    let result = application.start().await;

    assert!(matches!(result, Err(AgentError::Bind { .. })));
    std::fs::remove_dir_all(&dir).unwrap();
}

fn session(mac: &str, granted_at: i64, expires_at: i64, clock_untrusted: bool) -> Session {
    Session {
        mac: mac.to_owned(),
        token: "tok".to_owned(),
        venue: "venue-1".to_owned(),
        granted_at,
        expires_at,
        redirect_url: None,
        clock_untrusted,
    }
}

fn recording_ndsctl(dir: &std::path::Path) -> (Ndsctl, std::path::PathBuf) {
    let log = dir.join("calls.log");
    let program = dir.join("ndsctl");
    std::fs::write(
        &program,
        format!("#!/bin/sh\necho \"$@\" >> {}\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    (Ndsctl::with_program(program.to_str().unwrap()), log)
}

#[tokio::test]
async fn sweep_removes_expired_sessions_and_deauthenticates_them_in_opennds() {
    let dir = scratch_dir("sweep");
    let (ndsctl, log) = recording_ndsctl(&dir);
    let store = SessionStore::open_in_memory().unwrap();
    let now = build_unix() + 1000;
    store
        .upsert_session(session("aa:bb:cc:dd:ee:01", now - 100, now - 1, false))
        .await
        .unwrap();
    store
        .upsert_session(session("aa:bb:cc:dd:ee:02", now - 100, now + 500, false))
        .await
        .unwrap();

    let removed = sweep_once(&store, &ndsctl, now).await;

    assert_eq!(removed, 1);
    assert_eq!(
        std::fs::read_to_string(&log).unwrap().trim(),
        "deauth aa:bb:cc:dd:ee:01"
    );
    assert!(
        store
            .get_session("aa:bb:cc:dd:ee:02")
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(store.pending_event_count().await.unwrap(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn sweep_does_nothing_while_the_clock_is_untrusted() {
    let store = SessionStore::open_in_memory().unwrap();
    let before_build = build_unix() - 10;
    store
        .upsert_session(session("aa:bb:cc:dd:ee:01", 0, 5, false))
        .await
        .unwrap();

    let removed = sweep_once(&store, &Ndsctl::disabled(), before_build).await;

    assert_eq!(removed, 0);
    assert!(
        store
            .get_session("aa:bb:cc:dd:ee:01")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn sweep_restarts_sessions_granted_before_the_clock_was_set() {
    let store = SessionStore::open_in_memory().unwrap();
    store
        .upsert_session(session("aa:bb:cc:dd:ee:01", 100, 3700, true))
        .await
        .unwrap();
    let now = build_unix() + 10;

    let removed = sweep_once(&store, &Ndsctl::disabled(), now).await;

    assert_eq!(removed, 0);
    let got = store
        .get_session("aa:bb:cc:dd:ee:01")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.expires_at, now + 3600);
    assert!(!got.clock_untrusted);
}

#[tokio::test]
async fn healthcheck_succeeds_only_when_the_server_answers_200() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 256];
        let _ = stream.read(&mut request).await.unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .await
            .unwrap();
    });

    assert!(run_healthcheck(&addr.to_string()).await);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!run_healthcheck(&addr.to_string()).await);
}

#[tokio::test]
async fn healthcheck_fails_on_a_non_200_answer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 256];
        let _ = stream.read(&mut request).await.unwrap();
        stream
            .write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
            .await
            .unwrap();
    });

    assert!(!run_healthcheck(&addr.to_string()).await);
}

fn run_binauth_script(dir: &std::path::Path, args: &[&str]) -> String {
    let log = dir.join("fetch.log");
    let fake = dir.join("uclient-fetch");
    std::fs::write(
        &fake,
        format!("#!/bin/sh\necho \"$@\" >> {}\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let script = format!("{}/openwrt/custombinauth.sh", env!("CARGO_MANIFEST_DIR"));
    let path = format!("{}:/usr/bin:/bin", dir.display());

    let status = std::process::Command::new("/bin/sh")
        .arg(script)
        .args(args)
        .env("PATH", path)
        .status()
        .unwrap();
    assert!(status.success());
    std::thread::sleep(Duration::from_millis(300));
    std::fs::read_to_string(&log).unwrap_or_default()
}

#[test]
fn the_binauth_hook_forwards_deauthentications_to_the_agent_admin_listener() {
    let dir = scratch_dir("binauth-ok");

    let calls = run_binauth_script(&dir, &["timeout_deauth", "aa:bb:cc:dd:ee:ff", "0", "0"]);

    assert!(calls.contains("http://127.0.0.1:2081/binauth"));
    assert!(calls.contains(r#"{"mac":"aa:bb:cc:dd:ee:ff","method":"timeout_deauth"}"#));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_binauth_hook_ignores_authentications_and_malformed_macs() {
    let dir = scratch_dir("binauth-skip");

    let auth = run_binauth_script(&dir, &["auth_client", "aa:bb:cc:dd:ee:ff"]);
    let injected = run_binauth_script(&dir, &["client_deauth", "x\",\"method\":\"y"]);
    let short = run_binauth_script(&dir, &["client_deauth", "aa:bb"]);

    assert!(auth.is_empty());
    assert!(injected.is_empty());
    assert!(short.is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}
