#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use axum::Router;
use axum::http::{Method, StatusCode};
use ru5ty_gate_central_client::{
    CentralClient, ClientConfig, HeartbeatRequest, SyncEventDto, ValidateSessionRequest,
};
use ru5ty_gate_database::sqlx::PgPool;
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::net::TcpListener;

use crate::common::{
    RecordingEmailSender, UserFactory, build_application, call, config_with, live_redis,
    login_token, unique_name,
};

const T0: i64 = 1_790_000_000;

async fn router(pool: &PgPool) -> Router {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await
    .router()
}

async fn admin_token(app: &Router, pool: &PgPool, role: &'static str) -> String {
    let name = unique_name("Portal Admin");
    let (_, email) = UserFactory::new(pool, &name).role(role).create().await;
    login_token(app, &email).await
}

struct Provisioned {
    token: String,
    venue_id: String,
    api_key: String,
}

async fn provision(app: &Router, pool: &PgPool, code: &str) -> Provisioned {
    let token = admin_token(app, pool, "Super Admin").await;
    let (status, venue) = call(
        app,
        Method::POST,
        "/api/v1/venues",
        Some(&json!({
            "code": code,
            "name": format!("Venue {code}"),
            "sessionDurationSecs": 900,
            "redirectUrl": "https://venue.example/welcome",
        })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{venue}");
    let venue_id = venue["data"]["id"].as_str().unwrap().to_owned();
    let (status, gateway) = call(
        app,
        Method::POST,
        &format!("/api/v1/venues/{venue_id}/gateways"),
        Some(&json!({"name": "main"})),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{gateway}");
    Provisioned {
        token,
        venue_id,
        api_key: gateway["data"]["apiKey"].as_str().unwrap().to_owned(),
    }
}

fn heartbeat_body(venue: &str) -> Value {
    json!({
        "venue_id": venue,
        "uptime_secs": 120,
        "active_sessions": 2,
        "pending_events": 0,
        "last_sync_ok_at": T0,
        "agent_version": "1.0.0",
        "timestamp": T0,
    })
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn device_endpoints_reject_missing_and_wrong_keys(pool: PgPool) {
    let app = router(&pool).await;
    let provisioned = provision(&app, &pool, "venue-1").await;
    let validate = json!({"mac": "aa", "token": "t", "gateway_name": "g", "client_ip": "1.1.1.1"});

    let (anonymous, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/sessions/validate",
        Some(&validate),
        None,
    )
    .await;
    let (wrong, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/sessions/validate",
        Some(&validate),
        Some("not-a-key"),
    )
    .await;
    let admin_jwt_as_key = call(
        &app,
        Method::GET,
        "/api/v1/venues/venue-1/policy",
        None,
        Some(&provisioned.token),
    )
    .await
    .0;

    assert_eq!(anonymous, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong, StatusCode::UNAUTHORIZED);
    assert_eq!(admin_jwt_as_key, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn a_gateway_key_only_works_for_its_own_venue(pool: PgPool) {
    let app = router(&pool).await;
    let first = provision(&app, &pool, "venue-1").await;
    provision(&app, &pool, "venue-2").await;

    let (own, _) = call(
        &app,
        Method::GET,
        "/api/v1/venues/venue-1/policy",
        None,
        Some(&first.api_key),
    )
    .await;
    let (other, _) = call(
        &app,
        Method::GET,
        "/api/v1/venues/venue-2/policy",
        None,
        Some(&first.api_key),
    )
    .await;

    assert_eq!(own, StatusCode::OK);
    assert_eq!(other, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn validate_policy_heartbeat_and_sync_work_over_http(pool: PgPool) {
    let app = router(&pool).await;
    let provisioned = provision(&app, &pool, "venue-1").await;
    let key = Some(provisioned.api_key.as_str());

    let (status, validated) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/sessions/validate",
        Some(&json!({"mac": "aa:bb:cc:dd:ee:01", "token": "t", "gateway_name": "g", "client_ip": "10.0.0.5"})),
        key,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(validated["allow"], true);
    assert_eq!(validated["session_seconds"], 900);
    assert_eq!(validated["redirect_url"], "https://venue.example/welcome");

    let (_, policy) = call(
        &app,
        Method::GET,
        "/api/v1/venues/venue-1/policy",
        None,
        key,
    )
    .await;
    assert_eq!(policy["session_duration_secs"], 900);

    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/heartbeat",
        Some(&heartbeat_body("venue-1")),
        key,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, synced) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/sync",
        Some(&json!({"venue_id": "venue-1", "events": [{
            "event_type": "session_start",
            "payload": {"mac": "aa:bb:cc:dd:ee:01", "client_ip": "10.0.0.5", "granted_at": T0, "expires_at": T0 + 900},
            "occurred_at": T0,
        }]})),
        key,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(synced["accepted"], 1);

    let (_, sessions) = call(
        &app,
        Method::GET,
        &format!(
            "/api/v1/venues/{}/sessions?openOnly=true",
            provisioned.venue_id
        ),
        None,
        Some(&provisioned.token),
    )
    .await;
    assert_eq!(sessions["data"].as_array().unwrap().len(), 1);
    assert_eq!(sessions["data"][0]["macIdentifier"], "aa:bb:cc:dd:ee:01");

    let (_, gateways) = call(
        &app,
        Method::GET,
        &format!("/api/v1/venues/{}/gateways", provisioned.venue_id),
        None,
        Some(&provisioned.token),
    )
    .await;
    assert_eq!(gateways["data"][0]["agentVersion"], "1.0.0");
    assert_eq!(gateways["data"][0]["activeSessions"], 2);
    assert!(gateways["data"][0].get("apiKeyHash").is_none());
    assert!(gateways["data"][0].get("apiKey").is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn device_bodies_with_a_wrong_venue_or_unknown_fields_are_rejected(pool: PgPool) {
    let app = router(&pool).await;
    let provisioned = provision(&app, &pool, "venue-1").await;
    let key = Some(provisioned.api_key.as_str());

    let (mismatch, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/heartbeat",
        Some(&heartbeat_body("other-venue")),
        key,
    )
    .await;
    let mut extra = heartbeat_body("venue-1");
    extra["surprise"] = json!(true);
    let (unknown_field, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/heartbeat",
        Some(&extra),
        key,
    )
    .await;
    let (empty_mac, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/sessions/validate",
        Some(&json!({"mac": "", "token": "t", "gateway_name": "g", "client_ip": "1.1.1.1"})),
        key,
    )
    .await;
    let (too_many, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues/venue-1/sync",
        Some(&json!({"venue_id": "venue-1", "events": vec![json!({"event_type": "x", "payload": {}, "occurred_at": 1}); 1001]})),
        key,
    )
    .await;

    assert_eq!(mismatch, StatusCode::BAD_REQUEST);
    assert_eq!(unknown_field, StatusCode::BAD_REQUEST);
    assert_eq!(empty_mac, StatusCode::BAD_REQUEST);
    assert_eq!(too_many, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn rotating_a_key_over_http_locks_the_old_key_out(pool: PgPool) {
    let app = router(&pool).await;
    let provisioned = provision(&app, &pool, "venue-1").await;
    let (_, gateways) = call(
        &app,
        Method::GET,
        &format!("/api/v1/venues/{}/gateways", provisioned.venue_id),
        None,
        Some(&provisioned.token),
    )
    .await;
    let gateway_id = gateways["data"][0]["id"].as_str().unwrap();

    let (status, rotated) = call(
        &app,
        Method::POST,
        &format!("/api/v1/gateways/{gateway_id}/rotate-key"),
        None,
        Some(&provisioned.token),
    )
    .await;
    let new_key = rotated["data"]["apiKey"].as_str().unwrap().to_owned();
    let old = call(
        &app,
        Method::GET,
        "/api/v1/venues/venue-1/policy",
        None,
        Some(&provisioned.api_key),
    )
    .await
    .0;
    let fresh = call(
        &app,
        Method::GET,
        "/api/v1/venues/venue-1/policy",
        None,
        Some(&new_key),
    )
    .await
    .0;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(old, StatusCode::UNAUTHORIZED);
    assert_eq!(fresh, StatusCode::OK);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn venue_management_requires_the_matching_permission(pool: PgPool) {
    let app = router(&pool).await;
    let super_admin = admin_token(&app, &pool, "Super Admin").await;
    let support = admin_token(&app, &pool, "Support").await;
    let create = json!({"code": "venue-1", "name": "Venue"});

    let (anonymous, _) = call(&app, Method::GET, "/api/v1/venues", None, None).await;
    let (support_create, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&create),
        Some(&support),
    )
    .await;
    let (admin_create, created) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&create),
        Some(&super_admin),
    )
    .await;
    let (support_list, listed) =
        call(&app, Method::GET, "/api/v1/venues", None, Some(&support)).await;
    let id = created["data"]["id"].as_str().unwrap();
    let (support_update, _) = call(
        &app,
        Method::PUT,
        &format!("/api/v1/venues/{id}"),
        Some(&json!({"name": "Hacked"})),
        Some(&support),
    )
    .await;
    let (support_rotate, _) = call(
        &app,
        Method::POST,
        &format!("/api/v1/gateways/{}/rotate-key", uuid::Uuid::new_v4()),
        None,
        Some(&support),
    )
    .await;

    assert_eq!(anonymous, StatusCode::UNAUTHORIZED);
    assert_eq!(support_create, StatusCode::FORBIDDEN);
    assert_eq!(admin_create, StatusCode::CREATED);
    assert_eq!(support_list, StatusCode::OK);
    assert_eq!(listed["data"].as_array().unwrap().len(), 1);
    assert_eq!(support_update, StatusCode::FORBIDDEN);
    assert_eq!(support_rotate, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn venue_requests_are_validated_and_duplicates_conflict(pool: PgPool) {
    let app = router(&pool).await;
    let token = admin_token(&app, &pool, "Super Admin").await;

    let (bad_code, body) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&json!({"code": "has space", "name": "Venue"})),
        Some(&token),
    )
    .await;
    let (short_duration, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&json!({"code": "venue-1", "name": "Venue", "sessionDurationSecs": 5})),
        Some(&token),
    )
    .await;
    let (bad_url, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&json!({"code": "venue-1", "name": "Venue", "redirectUrl": "not a url"})),
        Some(&token),
    )
    .await;
    let (unknown, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&json!({"code": "venue-1", "name": "Venue", "extra": 1})),
        Some(&token),
    )
    .await;
    let valid = json!({"code": "venue-1", "name": "Venue"});
    call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&valid),
        Some(&token),
    )
    .await;
    let (duplicate, _) = call(
        &app,
        Method::POST,
        "/api/v1/venues",
        Some(&valid),
        Some(&token),
    )
    .await;
    let (missing, _) = call(
        &app,
        Method::GET,
        &format!("/api/v1/venues/{}", uuid::Uuid::new_v4()),
        None,
        Some(&token),
    )
    .await;

    assert_eq!(bad_code, StatusCode::BAD_REQUEST);
    assert!(
        body["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error["field"] == "code")
    );
    assert_eq!(short_duration, StatusCode::BAD_REQUEST);
    assert_eq!(bad_url, StatusCode::BAD_REQUEST);
    assert_eq!(unknown, StatusCode::BAD_REQUEST);
    assert_eq!(duplicate, StatusCode::CONFLICT);
    assert_eq!(missing, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn the_real_agent_client_works_against_the_service(pool: PgPool) {
    let app = router(&pool).await;
    let provisioned = provision(&app, &pool, "venue-1").await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = CentralClient::new(ClientConfig {
        base_url: format!("http://{address}/api"),
        api_key: Some(SecretString::from(provisioned.api_key.clone())),
        timeout: Duration::from_secs(5),
        venue_id: "venue-1".to_owned(),
    })
    .unwrap();

    let validated = client
        .validate_session(&ValidateSessionRequest {
            mac: "aa:bb:cc:dd:ee:01".to_owned(),
            token: "tok".to_owned(),
            gateway_name: "gw".to_owned(),
            client_ip: "10.0.0.5".to_owned(),
        })
        .await
        .unwrap();
    let policy = client.fetch_policy().await.unwrap();
    client
        .post_heartbeat(&HeartbeatRequest {
            venue_id: "venue-1".to_owned(),
            uptime_secs: 60,
            active_sessions: 1,
            pending_events: 0,
            last_sync_ok_at: None,
            agent_version: "1.0.0".to_owned(),
            timestamp: T0,
        })
        .await
        .unwrap();
    let synced = client
        .sync_events(vec![
            SyncEventDto {
                event_type: "session_start".to_owned(),
                payload: json!({"mac": "aa:bb:cc:dd:ee:01", "client_ip": "10.0.0.5", "granted_at": T0, "expires_at": T0 + 900}),
                occurred_at: T0,
            },
            SyncEventDto {
                event_type: "session_end".to_owned(),
                payload: json!({"mac": "aa:bb:cc:dd:ee:01", "reason": "expired", "ended_at": T0 + 900}),
                occurred_at: T0 + 900,
            },
        ])
        .await
        .unwrap();
    let rejected = CentralClient::new(ClientConfig {
        base_url: format!("http://{address}/api"),
        api_key: Some(SecretString::from("bogus".to_owned())),
        timeout: Duration::from_secs(5),
        venue_id: "venue-1".to_owned(),
    })
    .unwrap()
    .fetch_policy()
    .await;

    assert!(validated.allow);
    assert_eq!(validated.session_seconds, Some(900));
    assert_eq!(policy.session_duration_secs, 900);
    assert_eq!(synced.accepted, 2);
    assert!(rejected.is_err());
}
