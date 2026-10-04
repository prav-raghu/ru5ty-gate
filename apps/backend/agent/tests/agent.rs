#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use clap::Parser;
use ru5ty_gate_agent::{AgentError, Application, Cli};
use ru5ty_gate_agent_config::Settings;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ru5ty-gate-agent-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn settings_for(db_path: &str) -> Settings {
    let raw = format!(
        "[venue]\nid = \"venue-1\"\n\n[server]\nbind_addr = \"127.0.0.1:0\"\ndb_path = \"{db_path}\"\n"
    );
    Settings::parse(&raw, "inline").unwrap()
}

#[test]
fn cli_loads_settings_from_the_config_flag() {
    let dir = scratch_dir("cli");
    let path = dir.join("agent.toml");
    std::fs::write(
        &path,
        "[venue]\nid = \"venue-9\"\n\n[server]\nbind_addr = \"127.0.0.1:0\"\ndb_path = \"x.db\"\n",
    )
    .unwrap();

    let cli = Cli::parse_from(["ru5ty-gate-agent", "--config", path.to_str().unwrap()]);
    let settings = cli.load_settings().unwrap();

    assert_eq!(settings.venue.id, "venue-9");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cli_reports_an_error_when_the_config_file_is_missing() {
    let cli = Cli::parse_from(["ru5ty-gate-agent", "--config", "/nonexistent/agent.toml"]);

    assert!(cli.load_settings().is_err());
}

#[test]
fn initialize_opens_the_session_store_at_the_configured_path() {
    let dir = scratch_dir("init");
    let db_path = dir.join("sessions.db");

    let result = Application::initialize(settings_for(db_path.to_str().unwrap()));

    assert!(result.is_ok());
    assert!(db_path.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn initialize_fails_when_the_session_store_cannot_be_opened() {
    let dir = scratch_dir("blocked");
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, "not a directory").unwrap();
    let db_path = blocker.join("sessions.db");

    let result = Application::initialize(settings_for(db_path.to_str().unwrap()));

    assert!(matches!(result, Err(AgentError::Store { .. })));
    std::fs::remove_dir_all(&dir).unwrap();
}
