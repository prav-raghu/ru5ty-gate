#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_config::{ConfigError, EnvReader};

#[test]
fn required_returns_trimmed_value() {
    let reader = EnvReader::from_pairs([("DATABASE_URL", "  postgres://db  ")]);

    assert_eq!(
        reader.required("DATABASE_URL"),
        Ok("postgres://db".to_owned())
    );
}

#[test]
fn required_fails_when_missing_or_blank() {
    let reader = EnvReader::from_pairs([("REDIS_URL", "   ")]);

    assert_eq!(
        reader.required("REDIS_URL"),
        Err(ConfigError::Missing {
            key: "REDIS_URL".to_owned()
        })
    );
    assert_eq!(
        reader.required("JWT_SECRET"),
        Err(ConfigError::Missing {
            key: "JWT_SECRET".to_owned()
        })
    );
}

#[test]
fn parse_required_converts_value() {
    let reader = EnvReader::from_pairs([("PORT", "4002")]);

    assert_eq!(reader.parse_required::<u16>("PORT"), Ok(4002));
}

#[test]
fn parse_required_reports_invalid_value() {
    let reader = EnvReader::from_pairs([("PORT", "not-a-port")]);

    assert!(matches!(
        reader.parse_required::<u16>("PORT"),
        Err(ConfigError::Invalid { key, .. }) if key == "PORT"
    ));
}

#[test]
fn parse_or_uses_default_when_absent() {
    let reader = EnvReader::from_pairs([("OTHER", "1")]);

    assert_eq!(reader.parse_or::<u32>("RATE_LIMIT_MAX", 120), Ok(120));
}

#[test]
fn parse_or_rejects_invalid_override() {
    let reader = EnvReader::from_pairs([("RATE_LIMIT_MAX", "many")]);

    assert!(matches!(
        reader.parse_or::<u32>("RATE_LIMIT_MAX", 120),
        Err(ConfigError::Invalid { .. })
    ));
}

#[test]
fn optional_is_none_for_missing_key() {
    let reader = EnvReader::from_pairs([("A", "b")]);

    assert_eq!(reader.optional("MISSING"), None);
}
