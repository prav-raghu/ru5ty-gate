#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use ru5ty_gate_config::EnvReader;
use schedule_api::config::ServiceConfig;
use schedule_api::guards::ApiKey;

fn base() -> Vec<(&'static str, &'static str)> {
    vec![
        ("PORT", "4003"),
        ("CORS_ORIGIN", "http://localhost:4000"),
        ("REDIS_URL", "redis://localhost:6379"),
        ("DATABASE_URL", "postgres://x"),
        ("SCHEDULE_API_KEY", "0123456789abcdef0123456789abcdef"),
    ]
}

#[test]
fn loads_defaults() {
    let config = ServiceConfig::from_env(&EnvReader::from_pairs(base())).unwrap();

    assert_eq!(config.webhook_interval, Duration::from_secs(60));
    assert!(!config.production);
}

#[test]
fn rejects_weak_keys_missing_values_and_zero_intervals() {
    let mut weak = base();
    weak.retain(|(key, _)| *key != "SCHEDULE_API_KEY");
    weak.push(("SCHEDULE_API_KEY", "too-short"));
    let mut zero = base();
    zero.push(("SCHEDULE_WEBHOOK_INTERVAL_SECONDS", "0"));
    let without_port: Vec<_> = base()
        .into_iter()
        .filter(|(key, _)| *key != "PORT")
        .collect();

    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(weak)).is_err());
    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(zero)).is_err());
    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(without_port)).is_err());
}

#[test]
fn interval_can_be_overridden() {
    let mut env = base();
    env.push(("SCHEDULE_WEBHOOK_INTERVAL_SECONDS", "15"));

    assert_eq!(
        ServiceConfig::from_env(&EnvReader::from_pairs(env))
            .unwrap()
            .webhook_interval,
        Duration::from_secs(15)
    );
}

#[test]
fn api_key_comparison_requires_an_exact_match() {
    let key = ApiKey::new("0123456789abcdef0123456789abcdef");

    assert!(key.matches("0123456789abcdef0123456789abcdef"));
    assert!(!key.matches("0123456789abcdef0123456789abcdeX"));
    assert!(!key.matches("0123456789abcdef0123456789abcde"));
    assert!(!key.matches(""));
}
