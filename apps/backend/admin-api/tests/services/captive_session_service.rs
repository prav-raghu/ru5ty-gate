#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use admin_api::dtos::DeviceContext;
use admin_api::schemas::{
    CreateGatewayRequest, CreateVenueRequest, SessionListQuery, SyncBatchBody, SyncEventBody,
    UpdateVenueRequest, ValidateSessionBody,
};
use admin_api::services::CaptiveSessionService;
use ru5ty_gate_database::sqlx::PgPool;
use serde_json::json;
use uuid::Uuid;

use crate::common::{RecordingEmailSender, actor, build_application, config_with, live_redis};

struct Fixture {
    app: admin_api::application::Application,
    venue_id: Uuid,
    api_key: String,
    context: DeviceContext,
}

async fn fixture(pool: &PgPool, redirect_url: Option<&str>) -> Fixture {
    let app = build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await;
    let venue = app
        .state()
        .services
        .venue
        .create(
            &CreateVenueRequest {
                code: "venue-1".to_owned(),
                name: "Venue".to_owned(),
                session_duration_secs: Some(1800),
                redirect_url: redirect_url.map(str::to_owned),
                allow_new_sessions: None,
            },
            &actor("creator"),
        )
        .await
        .unwrap();
    let created = app
        .state()
        .services
        .gateway
        .create(
            venue.id,
            &CreateGatewayRequest {
                name: "main".to_owned(),
            },
            &actor("creator"),
        )
        .await
        .unwrap();
    let context = app
        .state()
        .services
        .gateway
        .authenticate(&created.api_key)
        .await
        .unwrap();
    Fixture {
        app,
        venue_id: venue.id,
        api_key: created.api_key,
        context,
    }
}

fn validate_body() -> ValidateSessionBody {
    ValidateSessionBody {
        mac: "aa:bb:cc:dd:ee:ff".to_owned(),
        token: "tok".to_owned(),
        gateway_name: "gw".to_owned(),
        client_ip: "10.0.0.5".to_owned(),
    }
}

fn start(mac_key: &str, mac: &str, granted: i64, expires: i64) -> SyncEventBody {
    SyncEventBody {
        event_type: "session_start".to_owned(),
        payload: json!({
            mac_key: mac,
            "venue": "venue-1",
            "gateway_name": "gw1",
            "client_ip": "10.0.0.5",
            "granted_at": granted,
            "expires_at": expires,
        }),
        occurred_at: granted,
    }
}

fn end(mac: &str, ended: i64, reason: &str) -> SyncEventBody {
    SyncEventBody {
        event_type: "session_end".to_owned(),
        payload: json!({"mac": mac, "reason": reason, "ended_at": ended}),
        occurred_at: ended,
    }
}

fn batch(events: Vec<SyncEventBody>) -> SyncBatchBody {
    SyncBatchBody {
        venue_id: "venue-1".to_owned(),
        events,
    }
}

const T0: i64 = 1_790_000_000;

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn validate_allows_with_the_venue_duration_and_redirect(pool: PgPool) {
    let fixture = fixture(&pool, Some("https://venue.example/welcome")).await;

    let response = CaptiveSessionService::validate(&fixture.context, &validate_body());

    assert!(response.allow);
    assert_eq!(response.session_seconds, Some(1800));
    assert_eq!(
        response.redirect_url.as_deref(),
        Some("https://venue.example/welcome")
    );
    assert!(response.reason.is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn validate_denies_while_the_venue_is_closed_to_new_sessions(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    fixture
        .app
        .state()
        .services
        .venue
        .update(
            fixture.venue_id,
            &UpdateVenueRequest {
                name: None,
                session_duration_secs: None,
                redirect_url: None,
                clear_redirect_url: None,
                allow_new_sessions: Some(false),
            },
            &actor("editor"),
        )
        .await
        .unwrap();
    let context = fixture
        .app
        .state()
        .services
        .gateway
        .authenticate(&fixture.api_key)
        .await
        .unwrap();

    let response = CaptiveSessionService::validate(&context, &validate_body());

    assert!(!response.allow);
    assert!(response.reason.is_some());
    assert!(response.session_seconds.is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn policy_reports_the_venue_defaults(pool: PgPool) {
    let fixture = fixture(&pool, Some("https://venue.example/welcome")).await;

    let policy = CaptiveSessionService::policy(&fixture.context);

    assert_eq!(policy.session_duration_secs, 1800);
    assert_eq!(
        policy.redirect_url.as_deref(),
        Some("https://venue.example/welcome")
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn sync_records_session_starts_and_ends(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    let service = &fixture.app.state().services.captive_session;

    let result = service
        .sync(
            &fixture.context,
            &batch(vec![
                start("mac", "AA:BB:CC:DD:EE:01", T0, T0 + 3600),
                end("aa:bb:cc:dd:ee:01", T0 + 600, "client_deauth"),
            ]),
        )
        .await
        .unwrap();

    assert_eq!(result.accepted, 2);
    let sessions = service
        .list(fixture.venue_id, &SessionListQuery::default())
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].mac_identifier, "aa:bb:cc:dd:ee:01");
    assert_eq!(sessions[0].client_identifier.as_deref(), Some("10.0.0.5"));
    assert_eq!(sessions[0].gateway_name.as_deref(), Some("gw1"));
    assert_eq!(sessions[0].end_reason.as_deref(), Some("client_deauth"));
    assert_eq!(sessions[0].ended_at.unwrap().timestamp(), T0 + 600);
    assert_eq!(sessions[0].expires_at.timestamp(), T0 + 3600);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn replaying_a_batch_does_not_duplicate_sessions(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    let service = &fixture.app.state().services.captive_session;
    let events = batch(vec![
        start("mac", "aa:bb:cc:dd:ee:01", T0, T0 + 3600),
        end("aa:bb:cc:dd:ee:01", T0 + 600, "expired"),
    ]);

    service.sync(&fixture.context, &events).await.unwrap();
    let replay = service.sync(&fixture.context, &events).await.unwrap();

    assert_eq!(replay.accepted, 2);
    let sessions = service
        .list(fixture.venue_id, &SessionListQuery::default())
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].end_reason.as_deref(), Some("expired"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn hashed_identifiers_from_privacy_mode_are_stored_as_sent(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    let service = &fixture.app.state().services.captive_session;
    let hash = "ab".repeat(32);
    let mut event = start("mac_hash", &hash, T0, T0 + 60);
    event.payload["client_ip_hash"] = json!("cd".repeat(32));
    event.payload.as_object_mut().unwrap().remove("client_ip");

    service
        .sync(&fixture.context, &batch(vec![event]))
        .await
        .unwrap();

    let sessions = service
        .list(fixture.venue_id, &SessionListQuery::default())
        .await
        .unwrap();
    assert_eq!(sessions[0].mac_identifier, hash);
    assert_eq!(
        sessions[0].client_identifier.as_deref(),
        Some("cd".repeat(32).as_str())
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn unusable_events_are_skipped_without_failing_the_batch(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    let service = &fixture.app.state().services.captive_session;
    let missing_expiry = SyncEventBody {
        event_type: "session_start".to_owned(),
        payload: json!({"mac": "aa:bb:cc:dd:ee:02", "granted_at": T0}),
        occurred_at: T0,
    };
    let inverted = start("mac", "aa:bb:cc:dd:ee:03", T0, T0 - 1);
    let no_mac = SyncEventBody {
        event_type: "session_start".to_owned(),
        payload: json!({"granted_at": T0, "expires_at": T0 + 60}),
        occurred_at: T0,
    };
    let unknown_type = SyncEventBody {
        event_type: "something_else".to_owned(),
        payload: json!({"mac": "aa:bb:cc:dd:ee:04"}),
        occurred_at: T0,
    };
    let oversized_mac = start("mac", &"a".repeat(200), T0, T0 + 60);
    let valid = start("mac", "aa:bb:cc:dd:ee:05", T0, T0 + 60);

    let result = service
        .sync(
            &fixture.context,
            &batch(vec![
                missing_expiry,
                inverted,
                no_mac,
                unknown_type,
                oversized_mac,
                valid,
            ]),
        )
        .await
        .unwrap();

    assert_eq!(result.accepted, 1);
    let sessions = service
        .list(fixture.venue_id, &SessionListQuery::default())
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].mac_identifier, "aa:bb:cc:dd:ee:05");
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn session_end_without_a_known_start_is_harmless(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    let service = &fixture.app.state().services.captive_session;

    let result = service
        .sync(
            &fixture.context,
            &batch(vec![end("aa:bb:cc:dd:ee:09", T0, "expired")]),
        )
        .await
        .unwrap();

    assert_eq!(result.accepted, 1);
    assert!(
        service
            .list(fixture.venue_id, &SessionListQuery::default())
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn list_can_show_only_open_sessions(pool: PgPool) {
    let fixture = fixture(&pool, None).await;
    let service = &fixture.app.state().services.captive_session;
    service
        .sync(
            &fixture.context,
            &batch(vec![
                start("mac", "aa:bb:cc:dd:ee:01", T0, T0 + 3600),
                start("mac", "aa:bb:cc:dd:ee:02", T0 + 10, T0 + 3600),
                end("aa:bb:cc:dd:ee:01", T0 + 100, "expired"),
            ]),
        )
        .await
        .unwrap();

    let open = service
        .list(
            fixture.venue_id,
            &SessionListQuery {
                open_only: Some(true),
                limit: None,
                offset: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(open.len(), 1);
    assert_eq!(open[0].mac_identifier, "aa:bb:cc:dd:ee:02");
}
