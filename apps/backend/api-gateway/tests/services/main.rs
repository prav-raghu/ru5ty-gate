#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../common/mod.rs"]
mod common;

use api_gateway::services::{HealthService, ServiceEndpoint};
use axum::http::StatusCode;
use ru5ty_gate_metrics::HealthStatus;

use crate::common::{DEAD_UPSTREAM, start_upstream};

fn endpoint(name: &str, url: &str) -> ServiceEndpoint {
    ServiceEndpoint {
        name: name.to_owned(),
        url: url.to_owned(),
        health_path: "/api/v1/ping".to_owned(),
    }
}

#[tokio::test]
async fn a_successful_probe_is_healthy_with_a_response_time() {
    let (url, _) = start_upstream("ok", StatusCode::OK).await;
    let service = HealthService::new(vec![endpoint("customer-api", &url)]);

    let result = service.check_service_by_name("customer-api").await.unwrap();

    assert_eq!(result.status, HealthStatus::Healthy);
    assert!(result.response_time_ms.is_some());
    assert!(result.error.is_none());
    assert!(service.is_healthy("customer-api").await);
}

#[tokio::test]
async fn http_errors_and_connection_failures_are_unhealthy_with_a_reason() {
    let (failing, _) = start_upstream("bad", StatusCode::SERVICE_UNAVAILABLE).await;
    let service = HealthService::new(vec![endpoint("a", &failing), endpoint("b", DEAD_UPSTREAM)]);

    let http_error = service.check_service_by_name("a").await.unwrap();
    let refused = service.check_service_by_name("b").await.unwrap();

    assert_eq!(http_error.status, HealthStatus::Unhealthy);
    assert_eq!(http_error.error.as_deref(), Some("HTTP 503"));
    assert_eq!(refused.status, HealthStatus::Unhealthy);
    assert!(refused.error.is_some());
}

#[tokio::test]
async fn aggregation_is_unhealthy_only_when_every_service_is_down() {
    let (healthy, _) = start_upstream("ok", StatusCode::OK).await;
    let mixed = HealthService::new(vec![endpoint("a", &healthy), endpoint("b", DEAD_UPSTREAM)]);
    let down = HealthService::new(vec![
        endpoint("a", DEAD_UPSTREAM),
        endpoint("b", DEAD_UPSTREAM),
    ]);
    let up = HealthService::new(vec![endpoint("a", &healthy)]);

    assert_eq!(
        mixed.check_all_services().await.status,
        HealthStatus::Degraded
    );
    assert_eq!(
        down.check_all_services().await.status,
        HealthStatus::Unhealthy
    );
    let all_up = up.check_all_services().await;
    assert_eq!(all_up.status, HealthStatus::Healthy);
    assert_eq!(all_up.services.len(), 1);
}

#[tokio::test]
async fn unknown_services_are_not_probed() {
    let service = HealthService::new(vec![endpoint("known", DEAD_UPSTREAM)]);

    assert!(service.check_service_by_name("unknown").await.is_none());
    assert!(!service.is_healthy("unknown").await);
    assert_eq!(service.service_names(), vec!["known"]);
}
