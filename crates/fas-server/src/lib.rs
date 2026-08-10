//! FAS (Forward Authentication Service) HTTP server.
//!
//! This is the part of the agent that actually talks to openNDS: a
//! lightweight axum endpoint openNDS calls with a client's MAC, its
//! per-attempt token, and the gateway identity. The agent decides
//! grant/deny (consulting the central platform, falling back to local
//! policy if it's unreachable) and, on grant, redirects the client's
//! browser back to openNDS's own auth endpoint to complete the round trip.

mod handlers;
pub mod opennds;

use axum::routing::get;
use axum::Router;

use central_client::CentralClient;
use session_store::SessionStore;

/// The subset of agent settings the FAS server needs to make grant/deny
/// decisions. Kept separate from `agent-config::Settings` so this crate
/// doesn't need to depend on the config crate -- the agent binary maps
/// `Settings` into this.
#[derive(Debug, Clone)]
pub struct FasConfig {
    pub venue_id: String,
    /// When set, requests carrying a different `gatewayname` are rejected.
    /// v1 is single-gateway by design.
    pub gateway_name: Option<String>,
    /// Fallback session duration used when the central platform doesn't
    /// supply one (or is unreachable and `allow_offline` is set).
    pub default_session_secs: u64,
    /// Whether to grant access using local policy when the central
    /// platform can't be reached at all.
    pub allow_offline: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub store: SessionStore,
    pub central: CentralClient,
    pub config: FasConfig,
}

/// Build the axum router: `/fas` for openNDS's auth callout, plus
/// `/health` and `/status` for operator diagnostics.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/fas", get(handlers::fas_auth))
        .route("/health", get(handlers::health))
        .route("/status", get(handlers::status))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use central_client::ClientConfig;
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let store = SessionStore::open_in_memory().unwrap();
        let central = CentralClient::new(ClientConfig {
            base_url: "http://127.0.0.1:1".to_string(), // nothing listens here -> forces offline fallback
            api_key: None,
            timeout: Duration::from_millis(200),
            venue_id: "venue-1".to_string(),
        })
        .unwrap();
        AppState {
            store,
            central,
            config: FasConfig {
                venue_id: "venue-1".to_string(),
                gateway_name: None,
                default_session_secs: 3600,
                allow_offline: true,
            },
        }
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn fas_auth_grants_and_redirects_when_central_unreachable_and_offline_allowed() {
        let app = router(test_state());
        let uri = "/fas?clientip=10.0.0.5&clientmac=AA:BB:CC:DD:EE:FF&gatewayname=gw1\
                   &gatewayaddress=10.0.0.1&gatewayport=2050&originurl=http://example.com\
                   &hid=abc123";
        let resp = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FOUND);
        let location = resp
            .headers()
            .get(axum::http::header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.starts_with("http://10.0.0.1:2050/opennds_auth/?"));
        assert!(location.contains("tok=abc123"));
    }

    #[tokio::test]
    async fn fas_auth_denies_when_offline_and_fail_closed() {
        let mut state = test_state();
        state.config.allow_offline = false;
        let app = router(state);
        let uri = "/fas?clientip=10.0.0.5&clientmac=AA:BB:CC:DD:EE:FF&gatewayname=gw1\
                   &gatewayaddress=10.0.0.1&gatewayport=2050&originurl=http://example.com\
                   &hid=abc123";
        let resp = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn fas_auth_rejects_unexpected_gateway() {
        let mut state = test_state();
        state.config.gateway_name = Some("expected-gw".to_string());
        let app = router(state);
        let uri = "/fas?clientip=10.0.0.5&clientmac=AA:BB:CC:DD:EE:FF&gatewayname=other-gw\
                   &gatewayaddress=10.0.0.1&gatewayport=2050&originurl=http://example.com\
                   &hid=abc123";
        let resp = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn status_reports_active_sessions() {
        let state = test_state();
        state
            .store
            .upsert_session(session_store::Session {
                mac: "AA:BB:CC:DD:EE:FF".to_string(),
                token: "tok".to_string(),
                venue: "venue-1".to_string(),
                granted_at: 0,
                expires_at: 9_999_999_999,
                redirect_url: None,
            })
            .await
            .unwrap();
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["active_sessions"], 1);
    }
}
