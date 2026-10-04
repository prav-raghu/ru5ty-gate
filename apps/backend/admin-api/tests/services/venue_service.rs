#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_http::AppError;
use uuid::Uuid;

use crate::common::{RecordingEmailSender, actor, build_application, config_with, live_redis};
use admin_api::schemas::{CreateVenueRequest, PageQuery, UpdateVenueRequest};

async fn app(pool: &PgPool) -> admin_api::application::Application {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await
}

fn create_request(code: &str) -> CreateVenueRequest {
    CreateVenueRequest {
        code: code.to_owned(),
        name: format!("Venue {code}"),
        session_duration_secs: None,
        redirect_url: None,
        allow_new_sessions: None,
    }
}

fn empty_update() -> UpdateVenueRequest {
    UpdateVenueRequest {
        name: None,
        session_duration_secs: None,
        redirect_url: None,
        clear_redirect_url: None,
        allow_new_sessions: None,
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn create_applies_defaults_and_attributes_the_actor(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.venue;

    let venue = service
        .create(&create_request("venue-1"), &actor("creator"))
        .await
        .unwrap();

    assert_eq!(venue.code, "venue-1");
    assert_eq!(venue.session_duration_secs, 3600);
    assert!(venue.allow_new_sessions);
    assert!(venue.redirect_url.is_none());
    let created_by: Option<String> =
        sqlx::query_scalar("SELECT created_by FROM venues WHERE id = $1")
            .bind(venue.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(created_by.as_deref(), Some("creator"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn create_rejects_a_duplicate_code_with_a_conflict(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.venue;
    service
        .create(&create_request("venue-1"), &actor("creator"))
        .await
        .unwrap();

    let error = service
        .create(&create_request("venue-1"), &actor("creator"))
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Conflict(_)));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn list_returns_active_venues_in_pages(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.venue;
    let mut ids = Vec::new();
    for code in ["venue-a", "venue-b", "venue-c"] {
        ids.push(
            service
                .create(&create_request(code), &actor("creator"))
                .await
                .unwrap()
                .id,
        );
    }
    sqlx::query("UPDATE venues SET is_active = FALSE WHERE id = $1")
        .bind(ids[1])
        .execute(&pool)
        .await
        .unwrap();

    let all = service.list(&PageQuery::default()).await.unwrap();
    let first_page = service
        .list(&PageQuery {
            limit: Some(1),
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|venue| venue.id != ids[1]));
    assert_eq!(first_page.len(), 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn update_changes_only_the_supplied_fields(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.venue;
    let created = service
        .create(
            &CreateVenueRequest {
                redirect_url: Some("https://venue.example/welcome".to_owned()),
                session_duration_secs: Some(1200),
                ..create_request("venue-1")
            },
            &actor("creator"),
        )
        .await
        .unwrap();

    let updated = service
        .update(
            created.id,
            &UpdateVenueRequest {
                name: Some("Renamed".to_owned()),
                allow_new_sessions: Some(false),
                ..empty_update()
            },
            &actor("editor"),
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Renamed");
    assert!(!updated.allow_new_sessions);
    assert_eq!(updated.session_duration_secs, 1200);
    assert_eq!(
        updated.redirect_url.as_deref(),
        Some("https://venue.example/welcome")
    );
    let modified_by: Option<String> =
        sqlx::query_scalar("SELECT modified_by FROM venues WHERE id = $1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(modified_by.as_deref(), Some("editor"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn update_can_clear_the_redirect_url(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.venue;
    let created = service
        .create(
            &CreateVenueRequest {
                redirect_url: Some("https://venue.example/welcome".to_owned()),
                ..create_request("venue-1")
            },
            &actor("creator"),
        )
        .await
        .unwrap();

    let updated = service
        .update(
            created.id,
            &UpdateVenueRequest {
                clear_redirect_url: Some(true),
                ..empty_update()
            },
            &actor("editor"),
        )
        .await
        .unwrap();

    assert!(updated.redirect_url.is_none());
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn get_and_update_report_not_found_for_unknown_or_inactive_venues(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.venue;
    let created = service
        .create(&create_request("venue-1"), &actor("creator"))
        .await
        .unwrap();
    sqlx::query("UPDATE venues SET is_active = FALSE WHERE id = $1")
        .bind(created.id)
        .execute(&pool)
        .await
        .unwrap();

    assert!(matches!(
        service.get(Uuid::new_v4()).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        service.get(created.id).await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        service
            .update(created.id, &empty_update(), &actor("editor"))
            .await,
        Err(AppError::NotFound(_))
    ));
    assert!(
        service
            .find_active_by_code("venue-1")
            .await
            .unwrap()
            .is_none()
    );
}
