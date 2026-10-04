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

const TOKEN: &str = "0123456789abcdef0123456789";

#[tokio::test]
async fn metrics_expose_prefixed_http_counters_without_counting_themselves() {
    let registry = REGISTRY
        .get_or_init(|| api_gateway::plugins::metrics::install_metrics().unwrap())
        .clone();
    let (customer, _) = start_upstream("customer", StatusCode::OK).await;
    let app = Application::with_parts(
        config(&customer, &customer, &customer, &[("METRICS_TOKEN", TOKEN)]),
        Some(registry),
    )
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
        .oneshot(
            Request::get("/metrics")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
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

async fn metrics_status(extra: &[(&str, &str)], authorization: Option<&str>) -> StatusCode {
    let registry = REGISTRY
        .get_or_init(|| api_gateway::plugins::metrics::install_metrics().unwrap())
        .clone();
    let (customer, _) = start_upstream("customer", StatusCode::OK).await;
    let app = Application::with_parts(
        config(&customer, &customer, &customer, extra),
        Some(registry),
    )
    .router();
    let mut request = Request::get("/metrics");
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }

    app.oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn metrics_are_not_served_unless_a_token_is_configured() {
    assert_eq!(metrics_status(&[], None).await, StatusCode::NOT_FOUND);
    assert_eq!(
        metrics_status(&[], Some(&format!("Bearer {TOKEN}"))).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn metrics_require_the_bearer_token() {
    let configured = [("METRICS_TOKEN", TOKEN)];

    assert_eq!(
        metrics_status(&configured, None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        metrics_status(&configured, Some("Bearer wrong-token-value-here")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        metrics_status(&configured, Some(TOKEN)).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        metrics_status(&configured, Some(&format!("Bearer {TOKEN}"))).await,
        StatusCode::OK
    );
}
