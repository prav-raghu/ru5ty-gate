use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl HealthCheckResult {
    pub fn new(status: HealthStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            latency: None,
            message: Some(message.into()),
            details: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: String,
    pub service: String,
    pub version: String,
    pub uptime: u64,
    pub checks: BTreeMap<String, HealthCheckResult>,
}
