#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_config::EnvReader;
use ru5ty_gate_observability::{init_sentry, resolve_sentry_config};

#[test]
fn disabled_without_dsn() {
    let config = resolve_sentry_config(&EnvReader::from_pairs([("APP_ENV", "production")]));

    assert!(!config.enabled);
    assert_eq!(config.environment, "production");
    assert!((config.traces_sample_rate - 0.1).abs() < f32::EPSILON);
}

#[test]
fn enabled_with_dsn_and_custom_rate() {
    let config = resolve_sentry_config(&EnvReader::from_pairs([
        ("SENTRY_DSN", "https://key@example.ingest.sentry.io/1"),
        ("SENTRY_TRACES_SAMPLE_RATE", "0.5"),
        ("SENTRY_RELEASE", "1.2.3"),
    ]));

    assert!(config.enabled);
    assert_eq!(config.release.as_deref(), Some("1.2.3"));
    assert!((config.traces_sample_rate - 0.5).abs() < f32::EPSILON);
}

#[test]
fn invalid_rate_falls_back_to_default() {
    let config =
        resolve_sentry_config(&EnvReader::from_pairs([("SENTRY_TRACES_SAMPLE_RATE", "x")]));

    assert!((config.traces_sample_rate - 0.1).abs() < f32::EPSILON);
}

#[test]
fn init_is_a_no_op_without_dsn() {
    assert!(init_sentry(&EnvReader::default()).is_none());
}
