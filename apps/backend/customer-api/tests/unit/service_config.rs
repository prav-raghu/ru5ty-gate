use customer_api::config::ServiceConfig;
use ru5ty_gate_config::EnvReader;

fn base() -> Vec<(&'static str, &'static str)> {
    vec![
        ("PORT", "4002"),
        ("CORS_ORIGIN", "http://localhost:3000"),
        ("CUSTOMER_WEB_URL", "http://localhost:3000"),
        ("REDIS_URL", "redis://localhost:6379"),
        ("DATABASE_URL", "postgres://x"),
        ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
        ("JWT_REFRESH_SECRET", "fedcba9876543210fedcba9876543210"),
    ]
}

#[test]
fn loads_a_complete_environment() {
    let config = ServiceConfig::from_env(&EnvReader::from_pairs(base())).unwrap();

    assert_eq!(config.port, 4002);
    assert!(!config.production);
    assert_eq!(config.database.max_connections, 10);
    assert!(config.redis_tls_reject_unauthorized);
}

#[test]
fn production_is_read_from_app_env() {
    let mut production = base();
    production.push(("APP_ENV", "production"));
    let mut node_env_only = base();
    node_env_only.push(("NODE_ENV", "production"));

    assert!(
        ServiceConfig::from_env(&EnvReader::from_pairs(production))
            .unwrap()
            .production
    );
    assert!(
        !ServiceConfig::from_env(&EnvReader::from_pairs(node_env_only))
            .unwrap()
            .production
    );
}

#[test]
fn rejects_unknown_environments_and_missing_required_values() {
    let mut staging = base();
    staging.push(("APP_ENV", "staging"));
    let without_port: Vec<_> = base()
        .into_iter()
        .filter(|(key, _)| *key != "PORT")
        .collect();
    let without_refresh: Vec<_> = base()
        .into_iter()
        .filter(|(key, _)| *key != "JWT_REFRESH_SECRET")
        .collect();

    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(staging)).is_err());
    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(without_port)).is_err());
    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(without_refresh)).is_err());
}

#[test]
fn rejects_non_numeric_ports() {
    let mut env = base();
    env.retain(|(key, _)| *key != "PORT");
    env.push(("PORT", "abc"));

    assert!(ServiceConfig::from_env(&EnvReader::from_pairs(env)).is_err());
}

#[test]
fn redis_tls_verification_is_only_disabled_by_an_explicit_false() {
    let verify = |value: Option<&'static str>| {
        let mut pairs = base();
        if let Some(value) = value {
            pairs.push(("REDIS_TLS_REJECT_UNAUTHORIZED", value));
        }
        ServiceConfig::from_env(&EnvReader::from_pairs(pairs))
            .unwrap()
            .redis_tls_reject_unauthorized
    };

    assert!(verify(None));
    assert!(verify(Some("true")));
    assert!(verify(Some("anything")));
    assert!(!verify(Some("false")));
}

#[test]
fn trusted_proxy_hops_defaults_to_one_and_can_be_overridden() {
    let hops = |value: Option<&'static str>| {
        let mut pairs = base();
        if let Some(value) = value {
            pairs.push(("TRUSTED_PROXY_HOPS", value));
        }
        ServiceConfig::from_env(&EnvReader::from_pairs(pairs))
            .unwrap()
            .trusted_proxy_hops
    };

    assert_eq!(hops(None), 1);
    assert_eq!(hops(Some("0")), 0);
    assert_eq!(hops(Some("2")), 2);
}
