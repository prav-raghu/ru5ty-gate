use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Duration, Utc};
use ru5ty_gate_database::{PgPool, sqlx};
use ru5ty_gate_export::{CsvExporter, ExcelExporter, ExportRow};
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::{
    ReportFilter, ReportFormat, ReportRequest, ReportResult, ReportStatus, ReportType,
};
use serde_json::Value;
use uuid::Uuid;

const WEBHOOK_REPORT_LIMIT: i64 = 10_000;

type DeliveryRecord = (
    Uuid,
    Uuid,
    String,
    String,
    Option<i32>,
    i32,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

#[derive(Clone)]
pub struct ReportingService {
    pool: PgPool,
}

fn row(pairs: Vec<(&str, Value)>) -> ExportRow {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn iso(date: DateTime<Utc>) -> Value {
    Value::String(date.to_rfc3339())
}

pub fn content_type(format: ReportFormat) -> &'static str {
    match format {
        ReportFormat::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "text/csv",
    }
}

fn sheet_name(report_type: ReportType) -> &'static str {
    match report_type {
        ReportType::UserActivity => "User Activity",
        ReportType::WebhookDelivery => "Webhook Deliveries",
        ReportType::SystemMetrics => "System Metrics",
        ReportType::AuditLog => "Audit Log",
        ReportType::Custom => "Report",
    }
}

impl ReportingService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn generate_report(&self, request: &ReportRequest) -> ReportResult {
        let id = format!(
            "report_{}_{}",
            Utc::now().timestamp_millis(),
            Uuid::new_v4().simple()
        );
        let failed = |error: String| ReportResult {
            id: id.clone(),
            report_type: request.report_type,
            format: request.format,
            status: ReportStatus::Failed,
            url: None,
            error: Some(error),
            record_count: None,
            generated_at: Some(Utc::now()),
        };
        let data = match self
            .fetch_report_data(request.report_type, request.filters.as_ref())
            .await
        {
            Ok(data) => data,
            Err(error) => return failed(error.to_string()),
        };
        let completed = |record_count: usize, url: Option<String>| ReportResult {
            id: id.clone(),
            report_type: request.report_type,
            format: request.format,
            status: ReportStatus::Completed,
            url,
            error: None,
            record_count: Some(record_count),
            generated_at: Some(Utc::now()),
        };
        if data.is_empty() {
            return completed(0, None);
        }
        match self.encode(request, &data) {
            Ok(bytes) => completed(
                data.len(),
                Some(format!(
                    "data:{};base64,{}",
                    content_type(request.format),
                    STANDARD.encode(bytes)
                )),
            ),
            Err(error) => failed(error.to_string()),
        }
    }

    pub fn encode(&self, request: &ReportRequest, data: &[ExportRow]) -> Result<Vec<u8>, AppError> {
        let headers: Option<Vec<String>> = if request.include_headers.unwrap_or(true) {
            data.first().map(|first| first.keys().cloned().collect())
        } else {
            None
        };
        match request.format {
            ReportFormat::Excel => ExcelExporter::new()
                .with_sheet_name(sheet_name(request.report_type))
                .export_to_bytes(data),
            _ => {
                let mut exporter = CsvExporter::new().with_bom(true);
                if let Some(headers) = headers {
                    exporter = exporter.with_headers(headers);
                }
                exporter.export_to_bytes(data)
            }
        }
        .map_err(AppError::internal)
    }

    pub async fn fetch_report_data(
        &self,
        report_type: ReportType,
        filters: Option<&ReportFilter>,
    ) -> Result<Vec<ExportRow>, AppError> {
        match report_type {
            ReportType::UserActivity => self.user_activity_report(filters).await,
            ReportType::WebhookDelivery => self.webhook_delivery_report(filters).await,
            ReportType::SystemMetrics => self.system_metrics_report(filters).await,
            ReportType::AuditLog => Ok(Vec::new()),
            ReportType::Custom => Err(AppError::BadRequest(
                "Unsupported report type: CUSTOM".to_owned(),
            )),
        }
    }

    pub async fn user_activity_report(
        &self,
        filters: Option<&ReportFilter>,
    ) -> Result<Vec<ExportRow>, AppError> {
        let start = filters
            .and_then(|filter| filter.start_date)
            .unwrap_or_else(|| Utc::now() - Duration::days(30));
        let end = filters
            .and_then(|filter| filter.end_date)
            .unwrap_or_else(Utc::now);
        let user_id = filters
            .and_then(|filter| filter.user_id.as_deref())
            .and_then(|value| Uuid::parse_str(value).ok());
        let status = filters.and_then(|filter| filter.status.as_deref());
        let records =
            sqlx::query_as::<_, (Uuid, String, String, Uuid, DateTime<Utc>, DateTime<Utc>)>(
                "SELECT id, email, username, user_status_id, created_at, updated_at FROM users \
             WHERE created_at >= $1 AND created_at <= $2 \
               AND ($3::uuid IS NULL OR id = $3) \
               AND ($4::text IS NULL OR user_status_id::text = $4) \
             ORDER BY created_at DESC",
            )
            .bind(start)
            .bind(end)
            .bind(user_id)
            .bind(status)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::internal)?;
        Ok(records
            .into_iter()
            .map(|(id, email, username, status_id, created_at, updated_at)| {
                row(vec![
                    ("userId", Value::String(id.to_string())),
                    ("email", Value::String(email)),
                    ("username", Value::String(username)),
                    ("status", Value::String(status_id.to_string())),
                    ("createdAt", iso(created_at)),
                    ("lastUpdated", iso(updated_at)),
                ])
            })
            .collect())
    }

    pub async fn webhook_delivery_report(
        &self,
        filters: Option<&ReportFilter>,
    ) -> Result<Vec<ExportRow>, AppError> {
        let start = filters
            .and_then(|filter| filter.start_date)
            .unwrap_or_else(|| Utc::now() - Duration::days(7));
        let end = filters
            .and_then(|filter| filter.end_date)
            .unwrap_or_else(Utc::now);
        let status = filters.and_then(|filter| filter.status.as_deref());
        let records = sqlx::query_as::<_, DeliveryRecord>(
            "SELECT id, subscription_id, event_type, status, http_status, attempt_count, \
             created_at, delivered_at FROM webhook_deliveries \
             WHERE created_at >= $1 AND created_at <= $2 AND ($3::text IS NULL OR status = $3) \
             ORDER BY created_at DESC LIMIT $4",
        )
        .bind(start)
        .bind(end)
        .bind(status)
        .bind(WEBHOOK_REPORT_LIMIT)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(records
            .into_iter()
            .map(
                |(
                    id,
                    subscription_id,
                    event_type,
                    status,
                    http_status,
                    attempts,
                    created,
                    delivered,
                )| {
                    row(vec![
                        ("deliveryId", Value::String(id.to_string())),
                        ("subscriptionId", Value::String(subscription_id.to_string())),
                        ("eventType", Value::String(event_type)),
                        ("status", Value::String(status)),
                        (
                            "httpStatus",
                            http_status
                                .map_or_else(|| Value::String("N/A".to_owned()), Value::from),
                        ),
                        ("attempts", Value::from(attempts)),
                        ("createdAt", iso(created)),
                        (
                            "deliveredAt",
                            delivered.map_or_else(|| Value::String("N/A".to_owned()), iso),
                        ),
                    ])
                },
            )
            .collect())
    }

    pub async fn system_metrics_report(
        &self,
        filters: Option<&ReportFilter>,
    ) -> Result<Vec<ExportRow>, AppError> {
        let start = filters
            .and_then(|filter| filter.start_date)
            .unwrap_or_else(|| Utc::now() - Duration::days(1));
        let end = filters
            .and_then(|filter| filter.end_date)
            .unwrap_or_else(Utc::now);
        let count = |query: &'static str| {
            let pool = self.pool.clone();
            async move {
                sqlx::query_scalar::<_, i64>(query)
                    .fetch_one(&pool)
                    .await
                    .map_err(AppError::internal)
            }
        };
        let total_users = count("SELECT COUNT(*) FROM users").await?;
        let subscriptions =
            count("SELECT COUNT(*) FROM webhook_subscriptions WHERE is_active = TRUE").await?;
        let active_users: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE is_active = TRUE AND updated_at >= $1",
        )
        .bind(start)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM webhook_deliveries \
             WHERE status = 'pending' AND created_at >= $1 AND created_at <= $2",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let now = iso(Utc::now());
        Ok([
            ("Total Users", total_users),
            ("Active Users", active_users),
            ("Webhook Subscriptions", subscriptions),
            ("Pending Webhook Deliveries", pending),
        ]
        .into_iter()
        .map(|(metric, value)| {
            row(vec![
                ("metric", Value::String(metric.to_owned())),
                ("value", Value::from(value)),
                ("timestamp", now.clone()),
            ])
        })
        .collect())
    }
}
