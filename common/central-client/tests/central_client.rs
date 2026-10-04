#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use secrecy::SecretString;

use ru5ty_gate_central_client::{
    CentralClient, CentralError, ClientConfig, HeartbeatRequest, IdentifierPolicy,
    ValidateSessionRequest,
};

fn unreachable_client() -> CentralClient {
    CentralClient::new(ClientConfig {
        base_url: "http://127.0.0.1:1".to_owned(),
        api_key: Some(SecretString::from("test-key".to_owned())),
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
        pending_events: 0,
        last_sync_ok_at: None,
        agent_version: "1.0.0".to_owned(),
        timestamp: 0,
    };

    let error = unreachable_client()
        .post_heartbeat(&heartbeat)
        .await
        .unwrap_err();

    assert!(matches!(error, CentralError::Request(_)));
}

#[test]
fn hashed_policy_replaces_raw_identifiers_in_event_payloads() {
    let policy = IdentifierPolicy::hashed(SecretString::from("0123456789abcdef".to_owned()));
    let payload =
        serde_json::json!({"mac": "aa:bb:cc:dd:ee:ff", "client_ip": "10.0.0.5", "venue": "v1"});

    let out = policy.apply_to_payload(payload);

    assert!(out.get("mac").is_none());
    assert!(out.get("client_ip").is_none());
    assert_eq!(out["venue"], "v1");
    assert_eq!(out["mac_hash"].as_str().unwrap().len(), 64);
    assert_eq!(out["client_ip_hash"].as_str().unwrap().len(), 64);
    assert!(!policy.sends_raw());
}

#[test]
fn raw_policy_leaves_payloads_untouched_and_masks_log_identifiers() {
    let policy = IdentifierPolicy::raw();
    let payload = serde_json::json!({"mac": "aa:bb:cc:dd:ee:ff"});

    assert_eq!(policy.apply_to_payload(payload.clone()), payload);
    assert_eq!(policy.mac("aa:bb:cc:dd:ee:ff"), "aa:bb:cc:dd:ee:ff");
    assert_eq!(policy.log_mac("AA:BB:CC:DD:EE:FF"), "aa:bb:cc:xx:xx:xx");
}

#[test]
fn hashed_policy_logs_a_short_hash_instead_of_the_mac() {
    let policy = IdentifierPolicy::hashed(SecretString::from("0123456789abcdef".to_owned()));

    let logged = policy.log_mac("aa:bb:cc:dd:ee:ff");

    assert_eq!(logged.len(), 12);
    assert!(!logged.contains(':'));
}
