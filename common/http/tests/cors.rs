#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, header};
use axum::routing::get;
use ru5ty_gate_http::cors_layer;
use tower::ServiceExt;

async fn allow_origin_for(layer_origins: &str, request_origin: &str) -> Option<String> {
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(cors_layer(layer_origins));
    let request = Request::builder()
        .uri("/")
        .header(header::ORIGIN, request_origin)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .map(|value| value.to_str().unwrap().to_owned())
}

#[tokio::test]
async fn a_single_configured_origin_is_allowed() {
    let allowed = allow_origin_for("http://localhost:4005", "http://localhost:4005").await;

    assert_eq!(allowed.as_deref(), Some("http://localhost:4005"));
}

#[tokio::test]
async fn every_origin_in_a_comma_separated_list_is_allowed() {
    let origins = "http://localhost:4005, http://localhost:4007/";

    assert_eq!(
        allow_origin_for(origins, "http://localhost:4005")
            .await
            .as_deref(),
        Some("http://localhost:4005")
    );
    assert_eq!(
        allow_origin_for(origins, "http://localhost:4007")
            .await
            .as_deref(),
        Some("http://localhost:4007")
    );
}

#[tokio::test]
async fn an_origin_outside_the_list_is_not_allowed() {
    let allowed = allow_origin_for(
        "http://localhost:4005,http://localhost:4007",
        "http://localhost:5173",
    )
    .await;

    assert_eq!(allowed, None);
}

#[tokio::test]
async fn an_empty_configuration_allows_no_origin() {
    let allowed = allow_origin_for("", "http://localhost:4005").await;

    assert_eq!(allowed, None);
}
