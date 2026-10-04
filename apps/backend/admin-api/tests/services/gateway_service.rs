#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use admin_api::schemas::{CreateGatewayRequest, CreateVenueRequest, HeartbeatBody};
use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_http::AppError;
use uuid::Uuid;

use crate::common::{RecordingEmailSender, actor, build_application, config_with, live_redis};

async fn app(pool: &PgPool) -> admin_api::application::Application {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await
}

async fn venue(app: &admin_api::application::Application, code: &str) -> Uuid {
    app.state()
        .services
        .venue
        .create(
            &CreateVenueRequest {
                code: code.to_owned(),
                name: format!("Venue {code}"),
                session_duration_secs: None,
                redirect_url: None,
                allow_new_sessions: None,
            },
            &actor("creator"),
        )
        .await
        .unwrap()
        .id
}

fn gateway_request(name: &str) -> CreateGatewayRequest {
    CreateGatewayRequest {
        name: name.to_owned(),
    }
}

fn heartbeat(venue_id: &str) -> HeartbeatBody {
    HeartbeatBody {
        venue_id: venue_id.to_owned(),
        uptime_secs: 7200,
        active_sessions: 3,
        pending_events: 1,
        last_sync_ok_at: Some(1_790_000_000),
        agent_version: "1.2.3".to_owned(),
        timestamp: 1_790_000_100,
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn create_returns_a_key_once_and_stores_only_its_hash(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;

    let created = service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap();

    let (id, secret) = created.api_key.split_once('.').unwrap();
    assert_eq!(id, created.gateway.id.to_string());
    assert!(secret.len() >= 64);
    let stored: String = sqlx::query_scalar("SELECT api_key_hash FROM gateways WHERE id = $1")
        .bind(created.gateway.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, secret);
    assert!(!stored.contains(secret));
    assert_eq!(stored.len(), 64);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn create_rejects_unknown_venues_and_duplicate_names(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;
    service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap();

    let duplicate = service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap_err();
    let unknown = service
        .create(Uuid::new_v4(), &gateway_request("x"), &actor("creator"))
        .await
        .unwrap_err();

    assert!(matches!(duplicate, AppError::Conflict(_)));
    assert!(matches!(unknown, AppError::NotFound(_)));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn authenticate_accepts_only_the_exact_issued_key(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;
    let created = service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap();
    let (id, secret) = created.api_key.split_once('.').unwrap();

    let context = service.authenticate(&created.api_key).await.unwrap();

    assert_eq!(context.gateway.id, created.gateway.id);
    assert_eq!(context.venue.code, "venue-1");
    for bad in [
        String::new(),
        "no-dot".to_owned(),
        format!("{id}."),
        format!("{id}.{secret}x"),
        format!("{id}.{}", &secret[1..]),
        format!("{}.{secret}", Uuid::new_v4()),
        format!("not-a-uuid.{secret}"),
        format!("{id}.{}", "a".repeat(500)),
    ] {
        assert!(service.authenticate(&bad).await.is_none(), "{bad}");
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn rotating_the_key_invalidates_the_old_one(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;
    let created = service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap();

    let rotated = service
        .rotate_key(created.gateway.id, &actor("rotator"))
        .await
        .unwrap();

    assert_ne!(rotated.api_key, created.api_key);
    assert!(service.authenticate(&created.api_key).await.is_none());
    assert!(service.authenticate(&rotated.api_key).await.is_some());
    assert!(matches!(
        service.rotate_key(Uuid::new_v4(), &actor("rotator")).await,
        Err(AppError::NotFound(_))
    ));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn deactivated_gateways_and_venues_cannot_authenticate(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;
    let first = service
        .create(venue_id, &gateway_request("a"), &actor("creator"))
        .await
        .unwrap();
    let second = service
        .create(venue_id, &gateway_request("b"), &actor("creator"))
        .await
        .unwrap();

    sqlx::query("UPDATE gateways SET is_active = FALSE WHERE id = $1")
        .bind(first.gateway.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(service.authenticate(&first.api_key).await.is_none());
    assert!(service.authenticate(&second.api_key).await.is_some());

    sqlx::query("UPDATE venues SET is_active = FALSE WHERE id = $1")
        .bind(venue_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(service.authenticate(&second.api_key).await.is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn heartbeats_are_recorded_on_the_gateway(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;
    let created = service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap();

    service
        .record_heartbeat(created.gateway.id, &heartbeat("venue-1"))
        .await
        .unwrap();

    let listed = service.list(venue_id).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].agent_version.as_deref(), Some("1.2.3"));
    assert_eq!(listed[0].uptime_secs, Some(7200));
    assert_eq!(listed[0].active_sessions, Some(3));
    assert_eq!(listed[0].pending_events, Some(1));
    assert!(listed[0].last_heartbeat_at.is_some());
    assert!(listed[0].last_sync_ok_at.is_some());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn out_of_range_heartbeat_counters_are_clamped_instead_of_failing(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.gateway;
    let venue_id = venue(&app, "venue-1").await;
    let created = service
        .create(venue_id, &gateway_request("main"), &actor("creator"))
        .await
        .unwrap();
    let mut extreme = heartbeat("venue-1");
    extreme.uptime_secs = u64::MAX;
    extreme.active_sessions = i64::MAX;
    extreme.pending_events = -1;

    service
        .record_heartbeat(created.gateway.id, &extreme)
        .await
        .unwrap();

    let listed = service.list(venue_id).await.unwrap();
    assert_eq!(listed[0].uptime_secs, Some(i64::MAX));
    assert_eq!(listed[0].active_sessions, Some(i32::MAX));
    assert_eq!(listed[0].pending_events, Some(-1));
}
