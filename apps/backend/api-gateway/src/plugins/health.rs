use std::sync::Arc;

use ru5ty_gate_metrics::{HealthCheckBuilder, HealthState};

use crate::config::ServiceConfig;
use crate::services::{HealthService, ServiceEndpoint};

const SERVICE_NAME: &str = "api-gateway";
const HEALTH_PATH: &str = "/api/v1/ping";

pub fn service_endpoints(config: &ServiceConfig) -> Vec<ServiceEndpoint> {
    [
        ("customer-api", &config.customer_api_url),
        ("admin-api", &config.admin_api_url),
        ("schedule-api", &config.scheduler_api_url),
    ]
    .into_iter()
    .map(|(name, url)| ServiceEndpoint {
        name: name.to_owned(),
        url: url.clone(),
        health_path: HEALTH_PATH.to_owned(),
    })
    .collect()
}

pub fn health_state(service: &Arc<HealthService>) -> HealthState {
    let probe = |name: &'static str| {
        let service = service.clone();
        move || {
            let service = service.clone();
            async move { service.is_healthy(name).await }
        }
    };
    let checks = HealthCheckBuilder::new()
        .add_external_service_check("customer-api", false, probe("customer-api"))
        .add_external_service_check("admin-api", false, probe("admin-api"))
        .build();
    HealthState::new(SERVICE_NAME, env!("CARGO_PKG_VERSION"), checks)
}
