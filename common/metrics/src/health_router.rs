use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::{SecondsFormat, Utc};
use futures_util::future::join_all;
use serde_json::{Value, json};

use crate::health_check::HealthCheckDefinition;
use crate::health_status::{HealthCheckResult, HealthResponse, HealthStatus};

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct HealthState {
    service_name: Arc<str>,
    version: Arc<str>,
    started: Instant,
    checks: Arc<[HealthCheckDefinition]>,
}

impl HealthState {
    pub fn new(service_name: &str, version: &str, checks: Vec<HealthCheckDefinition>) -> Self {
        Self {
            service_name: Arc::from(service_name),
            version: Arc::from(version),
            started: Instant::now(),
            checks: checks.into(),
        }
    }

    pub async fn run_checks(&self) -> HealthResponse {
        let outcomes = join_all(self.checks.iter().map(run_one)).await;
        let mut overall = HealthStatus::Healthy;
        let mut results = BTreeMap::new();
        for (definition, result) in self.checks.iter().zip(outcomes) {
            match result.status {
                HealthStatus::Unhealthy if definition.critical => overall = HealthStatus::Unhealthy,
                HealthStatus::Degraded if overall == HealthStatus::Healthy => {
                    overall = HealthStatus::Degraded;
                }
                _ => {}
            }
            results.insert(definition.name.clone(), result);
        }
        HealthResponse {
            status: overall,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            service: self.service_name.to_string(),
            version: self.version.to_string(),
            uptime: self.started.elapsed().as_secs(),
            checks: results,
        }
    }
}

async fn run_one(definition: &HealthCheckDefinition) -> HealthCheckResult {
    let started = Instant::now();
    let outcome = tokio::time::timeout(HEALTH_CHECK_TIMEOUT, (definition.check)()).await;
    let mut result = outcome.unwrap_or_else(|_| {
        HealthCheckResult::new(
            if definition.critical {
                HealthStatus::Unhealthy
            } else {
                HealthStatus::Degraded
            },
            "Health check timeout",
        )
    });
    result.latency = Some(started.elapsed().as_millis());
    result
}

async fn health(State(state): State<HealthState>) -> (StatusCode, Json<HealthResponse>) {
    let report = state.run_checks().await;
    let code = if report.status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    (code, Json(report))
}

async fn live(State(state): State<HealthState>) -> Json<Value> {
    Json(json!({
        "status": "alive",
        "timestamp": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        "service": state.service_name.as_ref(),
    }))
}

async fn ready(State(state): State<HealthState>) -> (StatusCode, Json<Value>) {
    let report = state.run_checks().await;
    let is_ready = report.status != HealthStatus::Unhealthy;
    let code = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = json!({
        "status": if is_ready { "ready" } else { "not_ready" },
        "timestamp": report.timestamp,
        "service": report.service,
        "checks": report.checks,
    });
    (code, Json(body))
}

pub fn health_router(state: HealthState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .with_state(state)
}
