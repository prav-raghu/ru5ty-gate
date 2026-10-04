#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::OnceLock;

use api_gateway::application::Application;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use ru5ty_gate_metrics::MetricsRegistry;
use tower::ServiceExt;

use crate::common::{config, start_upstream};

static REGISTRY: OnceLock<MetricsRegistry> = OnceLock::new();

#[tokio::test]
async fn metrics_expose_prefixed_http_counters_without_counting_themselves() {
    let registry = REGISTRY
        .get_or_init(|| api_gateway::plugins::metrics::install_metrics().unwrap())
        .clone();
    let (customer, _) = start_upstream("customer", StatusCode::OK).await;
    let app = Application::with_parts(config(&customer, &customer, &customer, &[]), Some(registry))
        .router();

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(Request::get("/api/v1/users").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let response = app
        .clone()
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let content_type = response.headers()["content-type"]
        .to_str()
        .unwrap()
        .to_owned();
    let text = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();

    assert!(content_type.starts_with("text/plain"));
    assert!(text.contains("gateway_http_requests_total"));
    assert!(text.contains("service=\"api-gateway\""));
    assert!(text.contains("status_code=\"200\""));
    assert!(!text.contains("route=\"/metrics\""));
}
