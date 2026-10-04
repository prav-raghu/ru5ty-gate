#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use api_gateway::config::{GraphqlConfig, ProxyTarget, ServiceConfig};
use api_gateway::controllers::upstream_url;
use ru5ty_gate_config::EnvReader;

fn base() -> Vec<(&'static str, &'static str)> {
    vec![
        ("PORT", "4000"),
        ("CORS_ORIGIN", "http://localhost:3000"),
        ("CUSTOMER_API_URL", "http://customer:4002/"),
        ("ADMIN_API_URL", "http://admin:4001"),
        ("SCHEDULER_API_URL", "http://schedule:4003"),
    ]
}

fn target(prefix: &'static str, strip: bool) -> ProxyTarget {
    ProxyTarget {
        name: "t",
        prefix,
        strip_prefix: strip,
        upstream: Arc::from("http://up:1"),
        client: reqwest::Client::new(),
    }
}

#[test]
fn urls_are_normalised_and_defaults_applied() {
    let config = ServiceConfig::from_env(&EnvReader::from_pairs(base())).unwrap();

    assert_eq!(config.customer_api_url, "http://customer:4002");
    assert_eq!(config.rate_limit_max, 200);
    assert!(!config.graphql.enabled);
    assert_eq!(config.graphql.path, "/graphql");
}

#[test]
fn every_upstream_url_is_required() {
    for missing in [
        "CUSTOMER_API_URL",
        "ADMIN_API_URL",
        "SCHEDULER_API_URL",
        "PORT",
        "CORS_ORIGIN",
    ] {
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

#[test]
fn graphql_dev_tools_are_disabled_in_production() {
    let env = EnvReader::from_pairs([
        ("GRAPHQL_ENABLED", "true"),
        ("GRAPHQL_PLAYGROUND", "true"),
        ("GRAPHQL_INTROSPECTION", "true"),
    ]);

    let development = GraphqlConfig::from_env(&env, false);
    let production = GraphqlConfig::from_env(&env, true);

    assert!(development.playground && development.introspection);
    assert!(production.enabled && !production.playground && !production.introspection);
}

#[test]
fn upstream_urls_follow_the_prefix_policy() {
    let customer = target("/api", false);
    let admin = target("/admin", true);

    assert_eq!(
        upstream_url(&customer, "/api/v1/ping", None),
        "http://up:1/api/v1/ping"
    );
    assert_eq!(
        upstream_url(&customer, "/api/v1/users", Some("a=1")),
        "http://up:1/api/v1/users?a=1"
    );
    assert_eq!(
        upstream_url(&admin, "/admin/api/v1/x", None),
        "http://up:1/api/v1/x"
    );
    assert_eq!(upstream_url(&admin, "/admin", None), "http://up:1/");
}
