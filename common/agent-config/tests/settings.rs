#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_agent_config::{ConfigError, Settings};
use secrecy::ExposeSecret;

const SECRET: &str = "0123456789abcdef0123456789abcdef";

fn minimal(extra: &str) -> String {
    format!(
        r#"
        [venue]
        id = "venue-1"

        [server]
        bind_addr = "0.0.0.0:2080"
        db_path = "/var/lib/ru5ty-gate/sessions.db"

        [fas]
        faskey = "{SECRET}"

        [privacy]
        hash_pepper = "{SECRET}"
        {extra}
    "#
    )
}

fn parse(raw: &str) -> Result<Settings, ConfigError> {
    Settings::parse(raw, "inline")
}

fn invalid_field(result: Result<Settings, ConfigError>) -> &'static str {
    match result {
        Err(ConfigError::Invalid { field, .. }) => field,
        other => panic!("expected an invalid config error, got {other:?}"),
    }
}

#[test]
fn parse_applies_defaults_to_a_minimal_config() {
    let settings = parse(&minimal("")).unwrap();

    assert_eq!(settings.venue.id, "venue-1");
    assert_eq!(settings.venue.name, "default");
    assert_eq!(settings.server.admin_bind_addr, "127.0.0.1:2081");
    assert_eq!(settings.session.default_duration_secs, 3600);
    assert!(settings.session.allow_offline);
    assert_eq!(settings.heartbeat.interval_secs, 60);
    assert_eq!(settings.sync.batch_size, 100);
    assert_eq!(settings.sync.max_pending, 50_000);
    assert_eq!(settings.sync.retain_synced_secs, 86_400);
    assert!(settings.fas.verify_client_ip);
    assert!(!settings.privacy.send_raw_identifiers);
    assert_eq!(settings.limits.fas_requests_per_minute, 30);
    assert!(settings.enforcement.ndsctl_path.is_none());
}

#[test]
fn parse_reads_every_section_of_a_full_config() {
    let raw = format!(
        r#"
        [venue]
        id = "venue-1"
        name = "Test Venue"
        gateway_name = "gl-mt6000"

        [server]
        bind_addr = "0.0.0.0:2080"
        admin_bind_addr = "127.0.0.1:2090"
        db_path = "sessions.db"

        [fas]
        faskey = "{SECRET}"
        verify_client_ip = false

        [central]
        base_url = "https://central.example.com"
        api_key = "{SECRET}"
        timeout_secs = 5

        [session]
        default_duration_secs = 7200
        allow_offline = false

        [heartbeat]
        interval_secs = 30

        [sync]
        interval_secs = 15
        batch_size = 50
        retain_synced_secs = 10
        max_pending = 500

        [privacy]
        send_raw_identifiers = true

        [limits]
        request_timeout_secs = 4
        max_concurrent_requests = 8
        fas_requests_per_minute = 5

        [enforcement]
        ndsctl_path = "/usr/bin/ndsctl"
    "#
    );

    let settings = parse(&raw).unwrap();

    assert_eq!(settings.venue.gateway_name.as_deref(), Some("gl-mt6000"));
    assert_eq!(settings.server.admin_bind_addr, "127.0.0.1:2090");
    assert!(!settings.fas.verify_client_ip);
    assert_eq!(settings.central.base_url, "https://central.example.com");
    assert_eq!(settings.central.timeout_secs, 5);
    assert!(!settings.session.allow_offline);
    assert_eq!(settings.sync.interval_secs, 15);
    assert_eq!(settings.limits.max_concurrent_requests, 8);
    assert_eq!(
        settings.enforcement.ndsctl_path.as_deref(),
        Some("/usr/bin/ndsctl")
    );
    assert_eq!(settings.fas.faskey.expose_secret(), SECRET);
}

#[test]
fn parse_fails_when_a_required_section_is_missing() {
    let result = parse("[venue]\nid = \"venue-1\"\n");

    assert!(matches!(result, Err(ConfigError::Parse { .. })));
}

#[test]
fn load_from_fails_when_the_file_does_not_exist() {
    let result = Settings::load_from("/nonexistent/ru5ty-gate/agent.toml");

    assert!(matches!(result, Err(ConfigError::Read { .. })));
}

#[test]
fn zero_intervals_and_timeouts_are_rejected() {
    assert_eq!(
        invalid_field(parse(&minimal("[heartbeat]\ninterval_secs = 0"))),
        "heartbeat.interval_secs"
    );
    assert_eq!(
        invalid_field(parse(&minimal("[sync]\ninterval_secs = 0"))),
        "sync.interval_secs"
    );
    assert_eq!(
        invalid_field(parse(&minimal("[central]\ntimeout_secs = 0"))),
        "central.timeout_secs"
    );
    assert_eq!(
        invalid_field(parse(&minimal("[session]\ndefault_duration_secs = 0"))),
        "session.default_duration_secs"
    );
    assert_eq!(
        invalid_field(parse(&minimal("[limits]\nrequest_timeout_secs = 0"))),
        "limits.request_timeout_secs"
    );
}

#[test]
fn batch_size_must_be_between_one_and_one_thousand() {
    assert_eq!(
        invalid_field(parse(&minimal("[sync]\nbatch_size = 0"))),
        "sync.batch_size"
    );
    assert_eq!(
        invalid_field(parse(&minimal(
            "[sync]\nbatch_size = 1001\nmax_pending = 5000"
        ))),
        "sync.batch_size"
    );
}

#[test]
fn max_pending_must_not_be_smaller_than_the_batch_size() {
    assert_eq!(
        invalid_field(parse(&minimal(
            "[sync]\nbatch_size = 100\nmax_pending = 10"
        ))),
        "sync.max_pending"
    );
}

#[test]
fn venue_id_must_be_a_safe_identifier() {
    for bad in ["", "has space", "slash/inside", &"a".repeat(65)] {
        let raw = minimal("").replace("id = \"venue-1\"", &format!("id = \"{bad}\""));

        assert_eq!(invalid_field(parse(&raw)), "venue.id");
    }
}

#[test]
fn bind_addresses_must_parse_and_differ() {
    let bad = minimal("").replace("0.0.0.0:2080", "not-an-address");
    let same = minimal("[server]\nadmin_bind_addr = \"0.0.0.0:2080\"");

    assert_eq!(invalid_field(parse(&bad)), "server.bind_addr");
    assert!(matches!(parse(&same), Err(ConfigError::Parse { .. })));
    let same = minimal("").replace(
        "db_path = \"/var/lib/ru5ty-gate/sessions.db\"",
        "db_path = \"x.db\"\n        admin_bind_addr = \"0.0.0.0:2080\"",
    );
    assert_eq!(invalid_field(parse(&same)), "server.admin_bind_addr");
}

#[test]
fn faskey_must_be_long_enough() {
    let raw = minimal("").replacen(SECRET, "short", 1);

    assert_eq!(invalid_field(parse(&raw)), "fas.faskey");
}

#[test]
fn api_key_over_plain_http_is_rejected_unless_explicitly_allowed() {
    let remote = minimal(&format!(
        "[central]\nbase_url = \"http://central.example.com\"\napi_key = \"{SECRET}\""
    ));
    let allowed = minimal(&format!(
        "[central]\nbase_url = \"http://central.example.com\"\napi_key = \"{SECRET}\"\nallow_insecure_http = true"
    ));
    let loopback = minimal(&format!(
        "[central]\nbase_url = \"http://127.0.0.1:8080\"\napi_key = \"{SECRET}\""
    ));
    let https = minimal(&format!(
        "[central]\nbase_url = \"https://central.example.com\"\napi_key = \"{SECRET}\""
    ));

    assert_eq!(invalid_field(parse(&remote)), "central.base_url");
    assert!(parse(&allowed).is_ok());
    assert!(parse(&loopback).is_ok());
    assert!(parse(&https).is_ok());
}

#[test]
fn base_url_must_be_http_or_https() {
    let raw = minimal("[central]\nbase_url = \"ftp://central.example.com\"");

    assert_eq!(invalid_field(parse(&raw)), "central.base_url");
}

#[test]
fn hash_pepper_is_required_unless_raw_identifiers_are_sent() {
    let without = minimal("").replace(&format!("hash_pepper = \"{SECRET}\""), "");
    let raw_ok = without.replace("[privacy]", "[privacy]\nsend_raw_identifiers = true");
    let short = minimal("")
        .replacen(SECRET, "short", 2)
        .replacen("short", SECRET, 1);

    assert_eq!(invalid_field(parse(&without)), "privacy.hash_pepper");
    assert!(parse(&raw_ok).is_ok());
    assert_eq!(invalid_field(parse(&short)), "privacy.hash_pepper");
}

#[test]
fn debug_output_never_contains_secrets() {
    let raw = minimal(&format!(
        "[central]\nbase_url = \"https://central.example.com\"\napi_key = \"{SECRET}\""
    ));

    let settings = parse(&raw).unwrap();

    assert!(!format!("{settings:?}").contains(SECRET));
}

const DEV_EXAMPLE: &str = include_str!("../../../apps/backend/agent/config/agent.example.toml");
const ROUTER_EXAMPLE: &str =
    include_str!("../../../apps/backend/agent/config/agent.router.example.toml");

#[test]
fn example_configs_are_valid_once_the_placeholders_are_replaced() {
    for example in [DEV_EXAMPLE, ROUTER_EXAMPLE] {
        let filled = example.replace("REPLACE_ME", SECRET);

        assert!(parse(&filled).is_ok());
    }
}

#[test]
fn example_configs_refuse_to_run_with_the_placeholders_left_in() {
    for example in [DEV_EXAMPLE, ROUTER_EXAMPLE] {
        assert!(matches!(
            parse(example),
            Err(ConfigError::Invalid {
                field: "fas.faskey",
                ..
            })
        ));
    }
}

#[test]
fn the_dev_example_listens_in_the_4000_port_range() {
    let settings = parse(&DEV_EXAMPLE.replace("REPLACE_ME", SECRET)).unwrap();

    assert_eq!(settings.server.bind_addr, "127.0.0.1:4009");
    assert_eq!(settings.server.admin_bind_addr, "127.0.0.1:4010");
}
