#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../common/mod.rs"]
mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::routing::{get, post};
use common::{Ports, RunningAgent, fas_path, http};
use tokio::net::TcpListener;

type Recorded = Arc<Mutex<Vec<(String, serde_json::Value)>>>;

async fn serve_central_on(port: u16, allow: bool, recorded: Recorded) {
    let validate = {
        let recorded = Arc::clone(&recorded);
        move |axum::Json(body): axum::Json<serde_json::Value>| {
            let recorded = Arc::clone(&recorded);
            async move {
                recorded.lock().unwrap().push(("validate".to_owned(), body));
                axum::Json(serde_json::json!({
                    "allow": allow,
                    "session_seconds": 600,
                    "reason": "venue closed",
                }))
            }
        }
    };
    let sync = {
        let recorded = Arc::clone(&recorded);
        move |axum::Json(body): axum::Json<serde_json::Value>| {
            let recorded = Arc::clone(&recorded);
            async move {
                recorded.lock().unwrap().push(("sync".to_owned(), body));
                axum::Json(serde_json::json!({"accepted": 1}))
            }
        }
    };
    let heartbeat = {
        let recorded = Arc::clone(&recorded);
        move |axum::Json(body): axum::Json<serde_json::Value>| {
            let recorded = Arc::clone(&recorded);
            async move {
                recorded
                    .lock()
                    .unwrap()
                    .push(("heartbeat".to_owned(), body));
            }
        }
    };
    let router = Router::new()
        .route("/v1/venues/venue-1/sessions/validate", post(validate))
        .route("/v1/venues/venue-1/sync", post(sync))
        .route("/v1/venues/venue-1/heartbeat", post(heartbeat))
        .route(
            "/v1/venues/venue-1/policy",
            get(|| async { axum::Json(serde_json::json!({"session_duration_secs": 900})) }),
        );
    let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
}

async fn wait_for(
    recorded: &Recorded,
    kind: &str,
    predicate: impl Fn(&serde_json::Value) -> bool,
) -> bool {
    for _ in 0..150 {
        let found = recorded
            .lock()
            .unwrap()
            .iter()
            .any(|(name, body)| name == kind && predicate(body));
        if found {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

fn has_event(body: &serde_json::Value, event_type: &str) -> bool {
    body["events"]
        .as_array()
        .is_some_and(|events| events.iter().any(|event| event["event_type"] == event_type))
}

#[tokio::test]
async fn a_client_is_granted_through_the_real_agent_and_its_session_events_reach_central() {
    let ports = Ports::allocate().await;
    let recorded: Recorded = Arc::default();
    serve_central_on(ports.central, true, Arc::clone(&recorded)).await;
    let agent = RunningAgent::start("e2e-grant", ports, "").await;

    let response = http(
        agent.public_addr(),
        "GET",
        &fas_path("AA:BB:CC:DD:EE:01", "hidone"),
        "",
    )
    .await;

    assert_eq!(response.status, 302);
    let location = response.location.unwrap();
    assert!(location.starts_with("http://10.0.0.1:2050/opennds_auth/?tok="));
    assert!(location.ends_with("&redir=http%3A%2F%2Fexample.com"));
    assert!(
        wait_for(&recorded, "validate", |body| body["mac"]
            == "aa:bb:cc:dd:ee:01")
        .await
    );
    assert!(wait_for(&recorded, "sync", |body| has_event(body, "session_start")).await);
    assert!(
        wait_for(&recorded, "heartbeat", |body| body["agent_version"]
            .is_string())
        .await
    );

    let status = http(agent.admin_addr(), "GET", "/status", "").await;
    assert_eq!(status.status, 200);
    assert!(status.body.contains("\"active_sessions\":1"));
    agent.stop();
}

#[tokio::test]
async fn deauthentication_reported_by_opennds_ends_the_session_and_is_synced() {
    let ports = Ports::allocate().await;
    let recorded: Recorded = Arc::default();
    serve_central_on(ports.central, true, Arc::clone(&recorded)).await;
    let agent = RunningAgent::start("e2e-binauth", ports, "").await;
    http(
        agent.public_addr(),
        "GET",
        &fas_path("AA:BB:CC:DD:EE:02", "hidtwo"),
        "",
    )
    .await;

    let ended = http(
        agent.admin_addr(),
        "POST",
        "/binauth",
        r#"{"mac":"aa:bb:cc:dd:ee:02","method":"client_deauth"}"#,
    )
    .await;

    assert_eq!(ended.status, 204);
    assert!(wait_for(&recorded, "sync", |body| has_event(body, "session_end")).await);
    let status = http(agent.admin_addr(), "GET", "/status", "").await;
    assert!(status.body.contains("\"active_sessions\":0"));
    agent.stop();
}

#[tokio::test]
async fn clients_are_granted_offline_and_buffered_events_sync_after_recovery() {
    let ports = Ports::allocate().await;
    let central_port = ports.central;
    let recorded: Recorded = Arc::default();
    let agent = RunningAgent::start("e2e-recovery", ports, "").await;

    let response = http(
        agent.public_addr(),
        "GET",
        &fas_path("AA:BB:CC:DD:EE:03", "hidthree"),
        "",
    )
    .await;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    assert_eq!(response.status, 302);
    assert!(recorded.lock().unwrap().is_empty());
    let status = http(agent.admin_addr(), "GET", "/status", "").await;
    assert!(status.body.contains("\"pending_sync_events\":1"));

    serve_central_on(central_port, true, Arc::clone(&recorded)).await;

    assert!(wait_for(&recorded, "sync", |body| has_event(body, "session_start")).await);
    agent.stop();
}

#[tokio::test]
async fn fail_closed_agents_deny_clients_while_central_is_down_and_obey_central_denials() {
    let ports = Ports::allocate().await;
    let central_port = ports.central;
    let recorded: Recorded = Arc::default();
    let agent = RunningAgent::start("e2e-closed", ports, "[session]\nallow_offline = false").await;

    let offline = http(
        agent.public_addr(),
        "GET",
        &fas_path("AA:BB:CC:DD:EE:04", "hidfour"),
        "",
    )
    .await;
    serve_central_on(central_port, false, Arc::clone(&recorded)).await;
    let denied = http(
        agent.public_addr(),
        "GET",
        &fas_path("AA:BB:CC:DD:EE:05", "hidfive"),
        "",
    )
    .await;

    assert_eq!(offline.status, 403);
    assert_eq!(denied.status, 403);
    assert!(denied.body.contains("venue closed"));
    agent.stop();
}

#[tokio::test]
async fn requests_that_do_not_follow_the_opennds_level_one_format_are_rejected() {
    let ports = Ports::allocate().await;
    let agent = RunningAgent::start("e2e-legacy", ports, "").await;

    let legacy = http(
        agent.public_addr(),
        "GET",
        "/fas?clientip=127.0.0.1&clientmac=AA:BB:CC:DD:EE:06&gatewayname=gw1&hid=abc&authaction=https://evil.example/",
        "",
    )
    .await;
    let health = http(agent.public_addr(), "GET", "/health", "").await;
    let admin_on_public = http(agent.public_addr(), "GET", "/status", "").await;

    assert_eq!(legacy.status, 400);
    assert_eq!(health.status, 200);
    assert_eq!(admin_on_public.status, 404);
    agent.stop();
}
