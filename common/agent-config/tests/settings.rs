#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_agent_config::{ConfigError, Settings};

#[test]
fn parse_applies_defaults_to_a_minimal_config() {
    let raw = r#"
        [venue]
        id = "venue-1"

        [server]
        bind_addr = "0.0.0.0:2080"
        db_path = "/var/lib/ru5ty-gate/sessions.db"
    "#;

    let settings = Settings::parse(raw, "inline").unwrap();

    assert_eq!(settings.venue.id, "venue-1");
    assert_eq!(settings.venue.name, "default");
    assert_eq!(settings.session.default_duration_secs, 3600);
    assert!(settings.session.allow_offline);
    assert_eq!(settings.heartbeat.interval_secs, 60);
    assert_eq!(settings.sync.batch_size, 100);
}

#[test]
fn parse_reads_every_section_of_a_full_config() {
    let raw = r#"
        [venue]
        id = "venue-1"
        name = "Test Venue"
        gateway_name = "gl-mt6000"

        [server]
        bind_addr = "0.0.0.0:2080"
        db_path = "sessions.db"

        [central]
        base_url = "https://central.example.com"
        api_key = "secret"
        timeout_secs = 5

        [session]
        default_duration_secs = 7200
        allow_offline = false

        [heartbeat]
        interval_secs = 30

        [sync]
        interval_secs = 15
        batch_size = 50
    "#;

    let settings = Settings::parse(raw, "inline").unwrap();

    assert_eq!(settings.venue.gateway_name.as_deref(), Some("gl-mt6000"));
    assert_eq!(settings.central.base_url, "https://central.example.com");
    assert_eq!(settings.central.api_key.as_deref(), Some("secret"));
    assert_eq!(settings.central.timeout_secs, 5);
    assert!(!settings.session.allow_offline);
    assert_eq!(settings.sync.interval_secs, 15);
}

#[test]
fn parse_fails_when_a_required_section_is_missing() {
    let result = Settings::parse("[venue]\nid = \"venue-1\"\n", "inline");

    assert!(matches!(result, Err(ConfigError::Parse { .. })));
}

#[test]
fn load_from_fails_when_the_file_does_not_exist() {
    let result = Settings::load_from("/nonexistent/ru5ty-gate/agent.toml");

    assert!(matches!(result, Err(ConfigError::Read { .. })));
}
