use admin_api::config::ServiceConfig;
use ru5ty_gate_config::EnvReader;

const KEY: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn base() -> Vec<(&'static str, &'static str)> {
    vec![
        ("PORT", "4001"),
        ("CORS_ORIGIN", "http://localhost:4004"),
        ("ADMIN_WEB_URL", "http://localhost:4004"),
        ("REDIS_URL", "redis://localhost:6379"),
        ("DATABASE_URL", "postgres://x"),
        ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
        ("JWT_REFRESH_SECRET", "fedcba9876543210fedcba9876543210"),
        ("TWO_FACTOR_ENCRYPTION_KEY", KEY),
    ]
}

#[test]
fn defaults_enable_bootstrap_and_use_a_30_minute_reset_window() {
    let config = ServiceConfig::from_env(&EnvReader::from_pairs(base())).unwrap();

    assert!(config.admin_bootstrap_enabled);
    assert_eq!(config.password_reset_expiration_minutes, 30);
    assert!(!config.production);
}

#[test]
fn bootstrap_can_be_disabled_with_the_kill_switch() {
    let mut env = base();
    env.push(("ADMIN_BOOTSTRAP_ENABLED", "false"));

    assert!(
        !ServiceConfig::from_env(&EnvReader::from_pairs(env))
            .unwrap()
            .admin_bootstrap_enabled
    );
}

#[test]
fn two_factor_key_must_be_64_hex_characters() {
    for bad in ["short", &"z".repeat(64), &"a".repeat(63)] {
        let mut env = base();
        env.retain(|(key, _)| *key != "TWO_FACTOR_ENCRYPTION_KEY");
        env.push(("TWO_FACTOR_ENCRYPTION_KEY", bad));
        assert!(
            ServiceConfig::from_env(&EnvReader::from_pairs(env)).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn requires_the_refresh_secret_and_admin_web_url() {
    for missing in ["JWT_REFRESH_SECRET", "ADMIN_WEB_URL", "PORT"] {
        let env: Vec<_> = base()
            .into_iter()
            .filter(|(key, _)| *key != missing)
            .collect();
        assert!(
            ServiceConfig::from_env(&EnvReader::from_pairs(env)).is_err(),
            "{missing}"
        );
    }
}
