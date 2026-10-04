#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use http_body_util::BodyExt;
use ru5ty_gate_central_client::{CentralClient, ClientConfig, IdentifierPolicy};
use ru5ty_gate_fas_server::{AppState, FasConfig, RouterLimits, admin_router, public_router};
use ru5ty_gate_session_store::{Session, SessionStore, StoredPolicy, SyncEventKind, unix_now};
use secrecy::SecretString;
use tokio::net::TcpListener;
use tower::ServiceExt;

const FASKEY: &str = "0123456789abcdef0123456789abcdef";
const CLIENT: &str = "10.0.0.5:51000";

fn payload(extra: &str) -> String {
    format!(
        "clientip=10.0.0.5, clientmac=AA:BB:CC:DD:EE:FF, gatewayname=gw1, hid=abc123hash, \
         gatewayaddress=10.0.0.1:2050, authdir=opennds_auth, originurl=http%3A%2F%2Fexample.com{extra}"
    )
}

fn uri_for(payload: &str) -> String {
    format!("/fas?fas={}", urlencoding_free(&STANDARD.encode(payload)))
}

fn urlencoding_free(base64: &str) -> String {
    base64
        .replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D")
}

fn state_with(base_url: &str) -> AppState {
    let central = CentralClient::new(ClientConfig {
        base_url: base_url.to_owned(),
        api_key: None,
        timeout: Duration::from_millis(300),
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
            faskey: SecretString::from(FASKEY.to_owned()),
            verify_client_ip: true,
        },
        identifiers: IdentifierPolicy::raw(),
    }
}

fn offline_state() -> AppState {
    state_with("http://127.0.0.1:1")
}

fn request(uri: &str, peer: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    }
    request
}

async fn call(router: Router, request: Request<Body>) -> axum::response::Response {
    router.oneshot(request).await.unwrap()
}

async fn fas(state: AppState, payload: &str, peer: Option<&str>) -> axum::response::Response {
    call(
        public_router(state, RouterLimits::default()),
        request(&uri_for(payload), peer),
    )
    .await
}

fn location(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

async fn serve(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{address}")
}

fn central_stub(body: serde_json::Value) -> Router {
    Router::new().route(
        "/v1/venues/venue-1/sessions/validate",
        axum::routing::post(move || {
            let body = body.clone();
            async move { axum::Json(body) }
        }),
    )
}

#[tokio::test]
async fn level_one_request_is_granted_and_redirected_with_the_derived_token() {
    let state = offline_state();

    let response = fas(state.clone(), &payload(""), Some(CLIENT)).await;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        location(&response),
        "http://10.0.0.1:2050/opennds_auth/?tok=9436d407c9db4628b0a6a3f534577d1b906be079c8a9f4dcb6d649882fe6a0a1&redir=http%3A%2F%2Fexample.com"
    );
    let session = state
        .store
        .get_session("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.token, "abc123hash");
    assert_eq!(state.store.pending_event_count().await.unwrap(), 1);
    let events = state.store.pending_events(10).await.unwrap();
    assert_eq!(events[0].kind, SyncEventKind::SessionStart);
    assert_eq!(events[0].payload["mac"], "aa:bb:cc:dd:ee:ff");
}

#[tokio::test]
async fn flat_legacy_parameters_without_a_fas_payload_are_rejected() {
    let uri = "/fas?clientip=10.0.0.5&clientmac=AA:BB:CC:DD:EE:FF&gatewayname=gw1&hid=abc&authaction=https://evil.example/";

    let response = call(
        public_router(offline_state(), RouterLimits::default()),
        request(uri, Some(CLIENT)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_and_injected_payloads_are_rejected_without_writing_anything() {
    let state = offline_state();

    let bad_authdir = payload("").replace("opennds_auth", "..%2Fadmin");
    let bad_gateway = payload("").replace("10.0.0.1:2050", "evil.example:80");
    for bad in [bad_authdir, bad_gateway, "not a payload".to_owned()] {
        let response = fas(state.clone(), &bad, Some(CLIENT)).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    assert_eq!(state.store.pending_event_count().await.unwrap(), 0);
    assert_eq!(state.store.active_session_count(0).await.unwrap(), 0);
}

#[tokio::test]
async fn a_request_from_a_different_peer_than_the_client_is_forbidden() {
    let state = offline_state();
    state
        .store
        .upsert_session(Session {
            mac: "aa:bb:cc:dd:ee:ff".to_owned(),
            token: "victim-token".to_owned(),
            venue: "venue-1".to_owned(),
            granted_at: 0,
            expires_at: 9_999_999_999,
            redirect_url: None,
            clock_untrusted: false,
        })
        .await
        .unwrap();

    let forged = fas(state.clone(), &payload(""), Some("10.0.0.99:51000")).await;
    let no_peer = fas(state.clone(), &payload(""), None).await;

    assert_eq!(forged.status(), StatusCode::FORBIDDEN);
    assert_eq!(no_peer.status(), StatusCode::FORBIDDEN);
    let session = state
        .store
        .get_session("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.token, "victim-token");
    assert_eq!(state.store.pending_event_count().await.unwrap(), 0);
}

#[tokio::test]
async fn peer_verification_can_be_switched_off_for_proxied_setups() {
    let mut state = offline_state();
    state.config.verify_client_ip = false;

    let response = fas(state, &payload(""), None).await;

    assert_eq!(response.status(), StatusCode::FOUND);
}

#[tokio::test]
async fn a_request_for_an_unexpected_gateway_is_rejected() {
    let mut state = offline_state();
    state.config.gateway_name = Some("expected-gw".to_owned());

    let response = fas(state, &payload(""), Some(CLIENT)).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn offline_clients_are_denied_when_the_policy_is_fail_closed() {
    let mut state = offline_state();
    state.config.allow_offline = false;

    let response = fas(state.clone(), &payload(""), Some(CLIENT)).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        state
            .store
            .get_session("aa:bb:cc:dd:ee:ff")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn the_central_decision_controls_duration_and_redirect() {
    let base_url = serve(central_stub(serde_json::json!({
        "allow": true,
        "session_seconds": 120,
        "redirect_url": "https://venue.example/welcome",
    })))
    .await;
    let state = state_with(&base_url);
    let before = unix_now();

    let response = fas(state.clone(), &payload(""), Some(CLIENT)).await;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(location(&response).ends_with("&redir=https%3A%2F%2Fvenue.example%2Fwelcome"));
    let session = state
        .store
        .get_session("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap()
        .unwrap();
    assert!(session.expires_at >= before + 120 && session.expires_at <= unix_now() + 120);
}

#[tokio::test]
async fn a_central_redirect_that_is_not_http_is_ignored() {
    let base_url = serve(central_stub(serde_json::json!({
        "allow": true,
        "redirect_url": "javascript:alert(1)",
    })))
    .await;

    let response = fas(state_with(&base_url), &payload(""), Some(CLIENT)).await;

    assert!(location(&response).ends_with("&redir=http%3A%2F%2Fexample.com"));
}

#[tokio::test]
async fn a_central_denial_is_forbidden_and_leaves_no_session() {
    let base_url = serve(central_stub(
        serde_json::json!({"allow": false, "reason": "banned"}),
    ))
    .await;
    let state = state_with(&base_url);

    let response = fas(state.clone(), &payload(""), Some(CLIENT)).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(state.store.active_session_count(0).await.unwrap(), 0);
}

#[tokio::test]
async fn the_cached_policy_applies_while_the_central_platform_is_unreachable() {
    let state = offline_state();
    state
        .store
        .save_policy(&StoredPolicy {
            session_duration_secs: 600,
            redirect_url: Some("https://venue.example/landing".to_owned()),
        })
        .await
        .unwrap();
    let before = unix_now();

    let response = fas(state.clone(), &payload(""), Some(CLIENT)).await;

    assert!(location(&response).ends_with("&redir=https%3A%2F%2Fvenue.example%2Flanding"));
    let session = state
        .store
        .get_session("aa:bb:cc:dd:ee:ff")
        .await
        .unwrap()
        .unwrap();
    assert!(session.expires_at >= before + 600 && session.expires_at <= unix_now() + 600);
}

#[tokio::test]
async fn hashed_identifiers_keep_raw_macs_out_of_central_requests() {
    use std::sync::{Arc, Mutex};
    let seen: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let recorder = Arc::clone(&seen);
    let router = Router::new().route(
        "/v1/venues/venue-1/sessions/validate",
        axum::routing::post(move |axum::Json(body): axum::Json<serde_json::Value>| {
            let recorder = Arc::clone(&recorder);
            async move {
                *recorder.lock().unwrap() = Some(body);
                axum::Json(serde_json::json!({"allow": true}))
            }
        }),
    );
    let base_url = serve(router).await;
    let mut state = state_with(&base_url);
    state.identifiers = IdentifierPolicy::hashed(SecretString::from(FASKEY.to_owned()));

    let response = fas(state, &payload(""), Some(CLIENT)).await;

    assert_eq!(response.status(), StatusCode::FOUND);
    let body = seen.lock().unwrap().clone().unwrap();
    assert_ne!(body["mac"], "aa:bb:cc:dd:ee:ff");
    assert_eq!(body["mac"].as_str().unwrap().len(), 64);
    assert_ne!(body["client_ip"], "10.0.0.5");
}

#[tokio::test]
async fn fas_requests_are_rate_limited_per_peer() {
    let router = public_router(
        offline_state(),
        RouterLimits {
            fas_requests_per_minute: 2,
            ..RouterLimits::default()
        },
    );
    let uri = uri_for(&payload(""));

    let mut statuses = Vec::new();
    for _ in 0..3 {
        let response = call(router.clone(), request(&uri, Some(CLIENT))).await;
        statuses.push(response.status());
    }
    let other = call(router.clone(), request(&uri, Some("10.0.0.5:1"))).await;

    assert_eq!(statuses[0], StatusCode::FOUND);
    assert_eq!(statuses[1], StatusCode::FOUND);
    assert_eq!(statuses[2], StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(other.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn health_is_public_and_carries_hardening_headers() {
    let response = call(
        public_router(offline_state(), RouterLimits::default()),
        request("/health", Some(CLIENT)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn status_and_binauth_are_not_served_on_the_public_listener() {
    let public = public_router(offline_state(), RouterLimits::default());

    let status = call(public.clone(), request("/status", Some(CLIENT))).await;
    let binauth = call(
        public,
        Request::builder()
            .method("POST")
            .uri("/binauth")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                r#"{"mac":"aa:bb:cc:dd:ee:ff","method":"client_deauth"}"#,
            ))
            .unwrap(),
    )
    .await;

    assert_eq!(status.status(), StatusCode::NOT_FOUND);
    assert_eq!(binauth.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn status_reports_sessions_queue_depth_and_clock_trust() {
    let state = offline_state();
    state
        .store
        .upsert_session(Session {
            mac: "aa:bb:cc:dd:ee:ff".to_owned(),
            token: "tok".to_owned(),
            venue: "venue-1".to_owned(),
            granted_at: 0,
            expires_at: 9_999_999_999,
            redirect_url: None,
            clock_untrusted: false,
        })
        .await
        .unwrap();

    let response = call(admin_router(state), request("/status", None)).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["active_sessions"], 1);
    assert_eq!(json["venue_id"], "venue-1");
    assert_eq!(json["clock_trusted"], true);
}

fn binauth(body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/binauth")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

#[tokio::test]
async fn binauth_deauthentication_ends_the_session_and_queues_an_event() {
    let state = offline_state();
    fas(state.clone(), &payload(""), Some(CLIENT)).await;

    let response = call(
        admin_router(state.clone()),
        binauth(r#"{"mac":"AA:BB:CC:DD:EE:FF","method":"timeout_deauth"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        state
            .store
            .get_session("aa:bb:cc:dd:ee:ff")
            .await
            .unwrap()
            .is_none()
    );
    let events = state.store.pending_events(10).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].kind, SyncEventKind::SessionEnd);
    assert_eq!(events[1].payload["reason"], "timeout_deauth");
}

#[tokio::test]
async fn binauth_ignores_authentication_methods_and_rejects_bad_macs() {
    let state = offline_state();
    fas(state.clone(), &payload(""), Some(CLIENT)).await;

    let auth = call(
        admin_router(state.clone()),
        binauth(r#"{"mac":"aa:bb:cc:dd:ee:ff","method":"client_auth"}"#),
    )
    .await;
    let bad = call(
        admin_router(state.clone()),
        binauth(r#"{"mac":"; rm -rf /","method":"client_deauth"}"#),
    )
    .await;

    assert_eq!(auth.status(), StatusCode::NO_CONTENT);
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    assert!(
        state
            .store
            .get_session("aa:bb:cc:dd:ee:ff")
            .await
            .unwrap()
            .is_some()
    );
}
