#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{Duration, Utc};
use ru5ty_gate_database::sqlx::{self, PgPool};
use ru5ty_gate_types::{ReportFilter, ReportFormat, ReportRequest, ReportStatus, ReportType};
use uuid::Uuid;

use crate::common::{
    RecordingEmailSender, UserFactory, build_application, config_with, live_redis, unique_name,
};

async fn app(pool: &PgPool) -> admin_api::application::Application {
    build_application(
        pool,
        RecordingEmailSender::new(),
        live_redis().await,
        config_with(&[]),
    )
    .await
}

fn request(report_type: ReportType, format: ReportFormat) -> ReportRequest {
    ReportRequest {
        report_type,
        format,
        filters: None,
        include_headers: Some(true),
        group_by: None,
    }
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn user_activity_report_filters_by_date_user_and_status(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.reporting;
    let (recent, _) = UserFactory::new(&pool, &unique_name("Recent"))
        .create()
        .await;
    let (old, _) = UserFactory::new(&pool, &unique_name("Old")).create().await;
    sqlx::query("UPDATE users SET created_at = NOW() - INTERVAL '90 days' WHERE id = $1")
        .bind(old)
        .execute(&pool)
        .await
        .unwrap();

    let default_window = service.user_activity_report(None).await.unwrap();
    let wide = ReportFilter {
        start_date: Some(Utc::now() - Duration::days(365)),
        ..ReportFilter::default()
    };
    let all = service.user_activity_report(Some(&wide)).await.unwrap();
    let one = ReportFilter {
        start_date: Some(Utc::now() - Duration::days(365)),
        user_id: Some(old.to_string()),
        ..ReportFilter::default()
    };
    let only_old = service.user_activity_report(Some(&one)).await.unwrap();

    assert_eq!(default_window.len(), 1);
    assert_eq!(default_window[0]["userId"], recent.to_string());
    assert_eq!(all.len(), 2);
    assert_eq!(only_old.len(), 1);
    assert!(only_old[0]["createdAt"].as_str().unwrap().contains('T'));
    let by_status = ReportFilter {
        status: Some(Uuid::new_v4().to_string()),
        ..ReportFilter::default()
    };
    assert!(
        service
            .user_activity_report(Some(&by_status))
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn webhook_delivery_report_uses_placeholders_for_missing_values(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.reporting;
    let subscription: Uuid = sqlx::query_scalar(
        "INSERT INTO webhook_subscriptions (url, secret) VALUES ('https://x.test', 's') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO webhook_deliveries (subscription_id, event_type, payload, status) \
         VALUES ($1, 'user.created', '{}', 'pending')",
    )
    .bind(subscription)
    .execute(&pool)
    .await
    .unwrap();

    let rows = service.webhook_delivery_report(None).await.unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["httpStatus"], "N/A");
    assert_eq!(rows[0]["deliveredAt"], "N/A");
    assert_eq!(rows[0]["status"], "pending");
    let filtered = ReportFilter {
        status: Some("delivered".to_owned()),
        ..ReportFilter::default()
    };
    assert!(
        service
            .webhook_delivery_report(Some(&filtered))
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn system_metrics_count_pending_deliveries_with_the_real_status_value(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.reporting;
    UserFactory::new(&pool, &unique_name("Metric Admin"))
        .create()
        .await;
    let subscription: Uuid = sqlx::query_scalar(
        "INSERT INTO webhook_subscriptions (url, secret) VALUES ('https://x.test', 's') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO webhook_deliveries (subscription_id, event_type, payload, status) \
         VALUES ($1, 'user.created', '{}', 'pending')",
    )
    .bind(subscription)
    .execute(&pool)
    .await
    .unwrap();

    let metrics = service.system_metrics_report(None).await.unwrap();

    let value = |name: &str| {
        metrics
            .iter()
            .find(|row| row["metric"] == name)
            .map(|row| row["value"].as_i64().unwrap())
            .unwrap()
    };
    assert_eq!(value("Total Users"), 1);
    assert_eq!(value("Active Users"), 1);
    assert_eq!(value("Webhook Subscriptions"), 1);
    assert_eq!(value("Pending Webhook Deliveries"), 1);
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn generated_csv_reports_are_data_uris_with_a_bom_and_headers(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.reporting;
    UserFactory::new(&pool, &unique_name("Csv Admin"))
        .create()
        .await;

    let result = service
        .generate_report(&request(ReportType::UserActivity, ReportFormat::Csv))
        .await;

    assert_eq!(result.status, ReportStatus::Completed);
    assert_eq!(result.record_count, Some(1));
    assert!(result.id.starts_with("report_"));
    let url = result.url.unwrap();
    let payload = url.strip_prefix("data:text/csv;base64,").unwrap();
    let bytes = STANDARD.decode(payload).unwrap();
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
    assert!(text.starts_with("createdAt,email,lastUpdated,status,userId,username\n"));
}

#[sqlx::test(migrations = "../../../common/database/migrations")]
async fn excel_empty_audit_and_unsupported_reports(pool: PgPool) {
    let app = app(&pool).await;
    let service = &app.state().services.reporting;
    UserFactory::new(&pool, &unique_name("Xlsx Admin"))
        .create()
        .await;

    let excel = service
        .generate_report(&request(ReportType::UserActivity, ReportFormat::Excel))
        .await;
    let audit = service
        .generate_report(&request(ReportType::AuditLog, ReportFormat::Csv))
        .await;
    let custom = service
        .generate_report(&request(ReportType::Custom, ReportFormat::Csv))
        .await;

    let url = excel.url.unwrap();
    assert!(url.starts_with(
        "data:application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;base64,"
    ));
    assert_eq!(audit.status, ReportStatus::Completed);
    assert_eq!(audit.record_count, Some(0));
    assert!(audit.url.is_none());
    assert_eq!(custom.status, ReportStatus::Failed);
    assert!(custom.error.unwrap().contains("Unsupported report type"));
}
