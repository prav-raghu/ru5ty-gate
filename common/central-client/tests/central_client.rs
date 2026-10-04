#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use ru5ty_gate_central_client::{
    CentralClient, CentralError, ClientConfig, HeartbeatRequest, ValidateSessionRequest,
};

fn unreachable_client() -> CentralClient {
    CentralClient::new(ClientConfig {
        base_url: "http://127.0.0.1:1".to_owned(),
        api_key: Some("test-key".to_owned()),
        timeout: Duration::from_secs(2),
        venue_id: "venue-1".to_owned(),
    })
    .unwrap()
}

#[tokio::test]
async fn validate_session_reports_a_request_error_when_the_server_is_unreachable() {
    let request = ValidateSessionRequest {
        mac: "AA:BB:CC:DD:EE:FF".to_owned(),
        token: "tok".to_owned(),
        gateway_name: "gw".to_owned(),
        client_ip: "10.0.0.5".to_owned(),
    };

    let error = unreachable_client()
        .validate_session(&request)
        .await
        .unwrap_err();

    assert!(matches!(error, CentralError::Request(_)));
}

#[tokio::test]
async fn post_heartbeat_reports_a_request_error_when_the_server_is_unreachable() {
    let heartbeat = HeartbeatRequest {
        venue_id: "venue-1".to_owned(),
        uptime_secs: 10,
        active_sessions: 0,
        agent_version: "1.0.0".to_owned(),
        timestamp: 0,
    };

    let error = unreachable_client()
        .post_heartbeat(&heartbeat)
        .await
        .unwrap_err();

    assert!(matches!(error, CentralError::Request(_)));
}
