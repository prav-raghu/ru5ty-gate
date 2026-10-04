#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use ru5ty_gate_central_client::{CentralClient, ClientConfig};
use ru5ty_gate_fas_server::{AppState, FasConfig, router};
use ru5ty_gate_session_store::{Session, SessionStore};
use tower::ServiceExt;

const FAS_URI: &str = "/fas?clientip=10.0.0.5&clientmac=AA:BB:CC:DD:EE:FF&gatewayname=gw1\
                       &gatewayaddress=10.0.0.1&gatewayport=2050&originurl=http://example.com\
                       &hid=abc123";

fn test_state() -> AppState {
    let central = CentralClient::new(ClientConfig {
        base_url: "http://127.0.0.1:1".to_owned(),
        api_key: None,
        timeout: Duration::from_millis(200),
        venue_id: "venue-1".to_owned(),
    })
    .unwrap();
    AppState {
        store: SessionStore::open_in_memory().unwrap(),
        central,
        config: FasConfig {
            venue_id: "venue-1".to_owned(),
            gateway_name: None,
            default_session_secs: 3600,
            allow_offline: true,
        },
    }
}

async fn get(state: AppState, uri: &str) -> axum::response::Response {
    router(state)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let response = get(test_state(), "/health").await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn fas_auth_grants_and_redirects_when_central_is_unreachable_and_offline_is_allowed() {
    let state = test_state();

    let response = get(state.clone(), FAS_URI).await;

    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with("http://10.0.0.1:2050/opennds_auth/?"));
    assert!(location.contains("tok=abc123"));
    assert!(
        state
            .store
            .get_session("AA:BB:CC:DD:EE:FF")
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(state.store.pending_event_count().await.unwrap(), 1);
}

#[tokio::test]
async fn fas_auth_denies_when_offline_and_fail_closed() {
    let mut state = test_state();
    state.config.allow_offline = false;

    let response = get(state.clone(), FAS_URI).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        state
            .store
            .get_session("AA:BB:CC:DD:EE:FF")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn fas_auth_rejects_unexpected_gateway() {
    let mut state = test_state();
    state.config.gateway_name = Some("expected-gw".to_owned());

    let response = get(state, FAS_URI).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn status_reports_active_sessions() {
    let state = test_state();
    state
        .store
        .upsert_session(Session {
            mac: "AA:BB:CC:DD:EE:FF".to_owned(),
            token: "tok".to_owned(),
            venue: "venue-1".to_owned(),
            granted_at: 0,
            expires_at: 9_999_999_999,
            redirect_url: None,
        })
        .await
        .unwrap();

    let response = get(state, "/status").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["active_sessions"], 1);
}
