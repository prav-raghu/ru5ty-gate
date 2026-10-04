use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use futures_util::future::join_all;
use reqwest::Client;
use ru5ty_gate_metrics::HealthStatus;
use serde::Serialize;

use crate::services::{ServiceEndpoint, ServiceHealth};

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
const SLOW_RESPONSE_MS: u128 = 3000;

#[derive(Debug, Clone, Serialize)]
pub struct GatewayHealth {
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GatewayHealthResponse {
    pub status: HealthStatus,
    pub timestamp: String,
    pub gateway: GatewayHealth,
    pub services: Vec<ServiceHealth>,
}

pub struct HealthService {
    endpoints: Vec<ServiceEndpoint>,
    client: Client,
    started: Instant,
}

impl HealthService {
    pub fn new(endpoints: Vec<ServiceEndpoint>) -> Self {
        Self {
            endpoints,
            client: Client::builder()
                .timeout(HEALTH_CHECK_TIMEOUT)
                .build()
                .unwrap_or_default(),
            started: Instant::now(),
        }
    }

    pub fn service_names(&self) -> Vec<&str> {
        self.endpoints
            .iter()
            .map(|endpoint| endpoint.name.as_str())
            .collect()
    }

    pub async fn check_all_services(&self) -> GatewayHealthResponse {
        let services = join_all(self.endpoints.iter().map(|endpoint| self.check(endpoint))).await;
        let unhealthy = services
            .iter()
            .filter(|service| service.status == HealthStatus::Unhealthy)
            .count();
        let degraded = services
            .iter()
            .filter(|service| service.status == HealthStatus::Degraded)
            .count();
        let status = if !services.is_empty() && unhealthy == services.len() {
            HealthStatus::Unhealthy
        } else if unhealthy > 0 || degraded > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        GatewayHealthResponse {
            status,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            gateway: GatewayHealth {
                uptime: self.started.elapsed().as_secs(),
            },
            services,
        }
    }

    pub async fn check_service_by_name(&self, name: &str) -> Option<ServiceHealth> {
        let endpoint = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.name == name)?;
        Some(self.check(endpoint).await)
    }

    async fn check(&self, endpoint: &ServiceEndpoint) -> ServiceHealth {
        let started = Instant::now();
        let outcome = self
            .client
            .get(format!("{}{}", endpoint.url, endpoint.health_path))
            .send()
            .await;
        let elapsed = started.elapsed().as_millis();
        let (status, error) = match outcome {
            Ok(response) if response.status().is_success() => (
                if elapsed > SLOW_RESPONSE_MS {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                },
                None,
            ),
            Ok(response) => {
                tracing::warn!(service = %endpoint.name, status = %response.status(), "Service health check failed");
                (
                    HealthStatus::Unhealthy,
                    Some(format!("HTTP {}", response.status().as_u16())),
                )
            }
            Err(error) => {
                tracing::error!(service = %endpoint.name, %error, "Service health check error");
                (HealthStatus::Unhealthy, Some(error.to_string()))
            }
        };
        ServiceHealth {
            name: endpoint.name.clone(),
            url: endpoint.url.clone(),
            status,
            response_time_ms: Some(elapsed),
            error,
        }
    }

    pub async fn is_healthy(&self, name: &str) -> bool {
        self.check_service_by_name(name)
            .await
            .is_some_and(|service| service.status == HealthStatus::Healthy)
    }
}
