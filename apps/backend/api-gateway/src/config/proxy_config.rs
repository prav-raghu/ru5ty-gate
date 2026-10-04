use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use reqwest::redirect::Policy;

use crate::config::ServiceConfig;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct ProxyTarget {
    pub name: &'static str,
    pub prefix: &'static str,
    pub strip_prefix: bool,
    pub upstream: Arc<str>,
    pub client: Client,
}

pub struct ProxyConfig;

impl ProxyConfig {
    fn client(connections: usize) -> Client {
        Client::builder()
            .redirect(Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .pool_max_idle_per_host(connections)
            .build()
            .unwrap_or_default()
    }

    pub fn targets(config: &ServiceConfig) -> Vec<ProxyTarget> {
        vec![
            ProxyTarget {
                name: "customer-api",
                prefix: "/api",
                strip_prefix: false,
                upstream: Arc::from(config.customer_api_url.as_str()),
                client: Self::client(128),
            },
            ProxyTarget {
                name: "admin-api",
                prefix: "/admin",
                strip_prefix: true,
                upstream: Arc::from(config.admin_api_url.as_str()),
                client: Self::client(64),
            },
            ProxyTarget {
                name: "schedule-api",
                prefix: "/scheduler",
                strip_prefix: true,
                upstream: Arc::from(config.scheduler_api_url.as_str()),
                client: Self::client(32),
            },
        ]
    }
}
