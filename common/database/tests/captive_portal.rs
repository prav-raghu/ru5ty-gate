#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Duration, Utc};
use ru5ty_gate_database::{
    CaptiveSessionRepository, Gateway, GatewayRepository, HeartbeatRecord, NewCaptiveSession,
    NewGateway, NewVenue, PgPool, Repository, SessionListFilter, Venue, VenueChanges,
    VenueRepository,
};
use uuid::Uuid;

fn new_venue(code: &str) -> NewVenue {
    NewVenue {
        code: code.to_owned(),
        name: format!("Venue {code}"),
        session_duration_secs: 1800,
        redirect_url: None,
        allow_new_sessions: true,
        created_by: "tester".to_owned(),
    }
}

async fn venue(pool: &PgPool, code: &str) -> Venue {
    Repository::<Venue>::insert(pool, &new_venue(code))
        .await
        .unwrap()
}

async fn gateway(pool: &PgPool, venue_id: Uuid, name: &str) -> Gateway {
    Repository::<Gateway>::insert(
        pool,
        &NewGateway {
            id: Uuid::new_v4(),
            venue_id,
            name: name.to_owned(),
            api_key_hash: "hash".to_owned(),
            created_by: "tester".to_owned(),
        },
    )
    .await
    .unwrap()
}

fn started(venue: &Venue, gateway: &Gateway, mac: &str, at: DateTime<Utc>) -> NewCaptiveSession {
    NewCaptiveSession {
        venue_id: venue.id,
        gateway_id: gateway.id,
        mac_identifier: mac.to_owned(),
        client_identifier: Some("10.0.0.5".to_owned()),
        gateway_name: Some("gw1".to_owned()),
        granted_at: at,
        expires_at: at + Duration::hours(1),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn venues_are_found_by_code_only_while_active(pool: PgPool) {
    let created = venue(&pool, "venue-1").await;

    let found = VenueRepository::find_active_by_code(&pool, "venue-1")
        .await
        .unwrap();
    assert_eq!(found.unwrap().id, created.id);
    assert!(
        VenueRepository::find_active_by_code(&pool, "missing")
            .await
            .unwrap()
            .is_none()
    );

    Repository::<Venue>::deactivate(&pool, created.id, "tester")
        .await
        .unwrap();
    assert!(
        VenueRepository::find_active_by_code(&pool, "venue-1")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn venue_codes_are_unique(pool: PgPool) {
    venue(&pool, "venue-1").await;

    let error = Repository::<Venue>::insert(&pool, &new_venue("venue-1"))
        .await
        .unwrap_err();

    assert!(error.is_unique_violation());
}

#[sqlx::test(migrations = "./migrations")]
async fn venue_changes_update_the_row_and_attribute_the_modifier(pool: PgPool) {
    let created = venue(&pool, "venue-1").await;

    let updated = Repository::<Venue>::update_by_id(
        &pool,
        created.id,
        &VenueChanges {
            name: "Renamed".to_owned(),
            session_duration_secs: 60,
            redirect_url: Some("https://venue.example".to_owned()),
            allow_new_sessions: false,
            modified_by: "editor".to_owned(),
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.session_duration_secs, 60);
    assert!(!updated.allow_new_sessions);
    assert_eq!(updated.modified_by.as_deref(), Some("editor"));
    assert_eq!(updated.created_by.as_deref(), Some("tester"));
    assert!(updated.updated_at >= created.updated_at);
}

#[sqlx::test(migrations = "./migrations")]
async fn session_durations_must_be_positive(pool: PgPool) {
    let mut invalid = new_venue("venue-1");
    invalid.session_duration_secs = 0;

    assert!(Repository::<Venue>::insert(&pool, &invalid).await.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn gateway_names_are_unique_per_venue(pool: PgPool) {
    let first = venue(&pool, "venue-1").await;
    let second = venue(&pool, "venue-2").await;
    gateway(&pool, first.id, "main").await;

    gateway(&pool, second.id, "main").await;
    let duplicate = Repository::<Gateway>::insert(
        &pool,
        &NewGateway {
            id: Uuid::new_v4(),
            venue_id: first.id,
            name: "main".to_owned(),
            api_key_hash: "hash".to_owned(),
            created_by: "tester".to_owned(),
        },
    )
    .await
    .unwrap_err();

    assert!(duplicate.is_unique_violation());
}

#[sqlx::test(migrations = "./migrations")]
async fn gateways_are_listed_per_venue_and_hidden_once_deactivated(pool: PgPool) {
    let first = venue(&pool, "venue-1").await;
    let second = venue(&pool, "venue-2").await;
    let kept = gateway(&pool, first.id, "b").await;
    let removed = gateway(&pool, first.id, "a").await;
    gateway(&pool, second.id, "other").await;
    Repository::<Gateway>::deactivate(&pool, removed.id, "tester")
        .await
        .unwrap();

    let listed = GatewayRepository::list_by_venue(&pool, first.id)
        .await
        .unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, kept.id);
    assert!(
        GatewayRepository::find_active(&pool, removed.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn replacing_the_key_hash_changes_only_that_gateway(pool: PgPool) {
    let venue = venue(&pool, "venue-1").await;
    let target = gateway(&pool, venue.id, "a").await;
    let other = gateway(&pool, venue.id, "b").await;

    let updated = GatewayRepository::replace_key_hash(&pool, target.id, "new-hash", "rotator")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.api_key_hash, "new-hash");
    assert_eq!(updated.modified_by.as_deref(), Some("rotator"));
    let untouched = GatewayRepository::find_active(&pool, other.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(untouched.api_key_hash, "hash");
}

#[sqlx::test(migrations = "./migrations")]
async fn heartbeats_update_the_gateway_status_columns(pool: PgPool) {
    let venue = venue(&pool, "venue-1").await;
    let target = gateway(&pool, venue.id, "a").await;
    let synced = Utc::now() - Duration::minutes(2);

    let recorded = GatewayRepository::record_heartbeat(
        &pool,
        target.id,
        &HeartbeatRecord {
            agent_version: "1.2.3".to_owned(),
            uptime_secs: 3600,
            active_sessions: 4,
            pending_events: 2,
            last_sync_ok_at: Some(synced),
        },
    )
    .await
    .unwrap();

    assert!(recorded);
    let stored = GatewayRepository::find_active(&pool, target.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.agent_version.as_deref(), Some("1.2.3"));
    assert_eq!(stored.uptime_secs, Some(3600));
    assert_eq!(stored.active_sessions, Some(4));
    assert_eq!(stored.pending_events, Some(2));
    assert!(stored.last_heartbeat_at.is_some());
    assert!(stored.last_sync_ok_at.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn recording_a_session_start_twice_stores_it_once(pool: PgPool) {
    let venue = venue(&pool, "venue-1").await;
    let gateway = gateway(&pool, venue.id, "a").await;
    let at = Utc::now();
    let session = started(&venue, &gateway, "aa:bb", at);

    let first = CaptiveSessionRepository::record_start(&pool, &session)
        .await
        .unwrap();
    let second = CaptiveSessionRepository::record_start(&pool, &session)
        .await
        .unwrap();

    assert!(first);
    assert!(!second);
    let filter = SessionListFilter {
        venue_id: venue.id,
        open_only: false,
        limit: 10,
        offset: 0,
    };
    assert_eq!(
        CaptiveSessionRepository::list(&pool, &filter)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_session_end_closes_the_latest_open_session_once(pool: PgPool) {
    let venue = venue(&pool, "venue-1").await;
    let gateway = gateway(&pool, venue.id, "a").await;
    let now = Utc::now();
    let older = started(&venue, &gateway, "aa:bb", now - Duration::hours(3));
    let newer = started(&venue, &gateway, "aa:bb", now - Duration::hours(1));
    CaptiveSessionRepository::record_start(&pool, &older)
        .await
        .unwrap();
    CaptiveSessionRepository::record_start(&pool, &newer)
        .await
        .unwrap();

    let first = CaptiveSessionRepository::record_end(&pool, gateway.id, "aa:bb", now, "expired")
        .await
        .unwrap();
    let replay = CaptiveSessionRepository::record_end(&pool, gateway.id, "aa:bb", now, "expired")
        .await
        .unwrap();
    let again = CaptiveSessionRepository::record_end(&pool, gateway.id, "aa:bb", now, "expired")
        .await
        .unwrap();

    assert!(first);
    assert!(replay);
    assert!(!again);
    let open = CaptiveSessionRepository::list(
        &pool,
        &SessionListFilter {
            venue_id: venue.id,
            open_only: true,
            limit: 10,
            offset: 0,
        },
    )
    .await
    .unwrap();
    assert!(open.is_empty());
    let all = CaptiveSessionRepository::list(
        &pool,
        &SessionListFilter {
            venue_id: venue.id,
            open_only: false,
            limit: 10,
            offset: 0,
        },
    )
    .await
    .unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].end_reason.as_deref(), Some("expired"));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_session_end_never_closes_a_session_that_starts_later(pool: PgPool) {
    let venue = venue(&pool, "venue-1").await;
    let gateway = gateway(&pool, venue.id, "a").await;
    let now = Utc::now();
    CaptiveSessionRepository::record_start(
        &pool,
        &started(&venue, &gateway, "aa:bb", now + Duration::hours(1)),
    )
    .await
    .unwrap();

    let closed = CaptiveSessionRepository::record_end(&pool, gateway.id, "aa:bb", now, "expired")
        .await
        .unwrap();

    assert!(!closed);
}

#[sqlx::test(migrations = "./migrations")]
async fn session_lists_are_scoped_to_the_venue_and_paginated(pool: PgPool) {
    let first = venue(&pool, "venue-1").await;
    let second = venue(&pool, "venue-2").await;
    let first_gateway = gateway(&pool, first.id, "a").await;
    let second_gateway = gateway(&pool, second.id, "a").await;
    let now = Utc::now();
    for minutes in 0..3 {
        CaptiveSessionRepository::record_start(
            &pool,
            &started(
                &first,
                &first_gateway,
                "aa:bb",
                now - Duration::minutes(minutes),
            ),
        )
        .await
        .unwrap();
    }
    CaptiveSessionRepository::record_start(&pool, &started(&second, &second_gateway, "cc:dd", now))
        .await
        .unwrap();

    let page = CaptiveSessionRepository::list(
        &pool,
        &SessionListFilter {
            venue_id: first.id,
            open_only: false,
            limit: 2,
            offset: 1,
        },
    )
    .await
    .unwrap();

    assert_eq!(page.len(), 2);
    assert!(page.iter().all(|session| session.venue_id == first.id));
    assert!(page[0].granted_at > page[1].granted_at);
}
