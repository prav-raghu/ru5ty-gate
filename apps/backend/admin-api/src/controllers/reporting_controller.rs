use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use futures_util::stream;
use ru5ty_gate_export::{CsvExporter, ExportRow};
use ru5ty_gate_http::{ApiQuery, AppError, ValidatedJson};
use ru5ty_gate_types::{ReportFilter, ReportFormat, ReportRequest, ReportStatus, ReportType};
use serde_json::json;

use crate::schemas::{
    GenerateReportRequest, ReportFilterInput, ReportFormatInput, ReportKind, StreamFormat,
    StreamReportQuery, SystemMetricsQuery, UserActivityQuery, WebhookDeliveryQuery,
};
use crate::services::reporting_content_type;
use crate::types::AppState;

fn report_type(kind: ReportKind) -> ReportType {
    match kind {
        ReportKind::UserActivity => ReportType::UserActivity,
        ReportKind::SystemMetrics => ReportType::SystemMetrics,
        ReportKind::AuditLog => ReportType::AuditLog,
        ReportKind::WebhookDelivery => ReportType::WebhookDelivery,
    }
}

fn report_format(format: ReportFormatInput) -> ReportFormat {
    match format {
        ReportFormatInput::Csv => ReportFormat::Csv,
        ReportFormatInput::Excel => ReportFormat::Excel,
        ReportFormatInput::Json => ReportFormat::Json,
        ReportFormatInput::Pdf => ReportFormat::Pdf,
    }
}

fn filter_from(input: Option<ReportFilterInput>) -> Option<ReportFilter> {
    input.map(|filters| ReportFilter {
        start_date: filters.start_date,
        end_date: filters.end_date,
        user_id: filters.user_id.map(|id| id.to_string()),
        status: filters.status,
        extra: serde_json::Map::new(),
    })
}

pub struct ReportingController;

impl ReportingController {
    pub async fn generate_report(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<GenerateReportRequest>,
    ) -> Result<Response, AppError> {
        let request = ReportRequest {
            report_type: report_type(body.report_type),
            format: report_format(body.format),
            filters: filter_from(body.filters),
            include_headers: Some(body.include_headers.unwrap_or(true)),
            group_by: None,
        };
        let result = state.services.reporting.generate_report(&request).await;
        let successful = result.status == ReportStatus::Completed;
        Ok((
            StatusCode::OK,
            Json(json!({ "isSuccessful": successful, "data": result })),
        )
            .into_response())
    }

    pub async fn stream_report(
        State(state): State<AppState>,
        ApiQuery(query): ApiQuery<StreamReportQuery>,
    ) -> Result<Response, AppError> {
        let format = match query.format {
            StreamFormat::Csv => ReportFormat::Csv,
            StreamFormat::Excel => ReportFormat::Excel,
        };
        let kind = report_type(query.report_type);
        let filters = ReportFilter {
            start_date: query.start_date,
            end_date: query.end_date,
            ..ReportFilter::default()
        };
        let rows = state
            .services
            .reporting
            .fetch_report_data(kind, Some(&filters))
            .await?;
        let extension = if format == ReportFormat::Excel {
            ".xlsx"
        } else {
            ".csv"
        };
        let name = format!(
            "attachment; filename=\"report-{}-{}{extension}\"",
            serde_json::to_value(query.report_type)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default(),
            Utc::now().timestamp_millis()
        );

        let body = if format == ReportFormat::Excel {
            let request = ReportRequest {
                report_type: kind,
                format,
                filters: Some(filters),
                include_headers: Some(true),
                group_by: None,
            };
            Body::from(state.services.reporting.encode(&request, &rows)?)
        } else {
            Body::from_stream(csv_stream(&rows))
        };
        let mut response = (StatusCode::OK, body).into_response();
        let headers = response.headers_mut();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(reporting_content_type(format)),
        );
        if let Ok(value) = HeaderValue::from_str(&name) {
            headers.insert(header::CONTENT_DISPOSITION, value);
        }
        Ok(response)
    }

    pub async fn get_user_activity_report(
        State(state): State<AppState>,
        ApiQuery(query): ApiQuery<UserActivityQuery>,
    ) -> Result<Response, AppError> {
        let filters = filter_from(Some(ReportFilterInput {
            start_date: query.start_date,
            end_date: query.end_date,
            user_id: query.user_id,
            status: query.status,
        }));
        let records = state
            .services
            .reporting
            .user_activity_report(filters.as_ref())
            .await?;
        Ok(records_response("records", &records))
    }

    pub async fn get_webhook_delivery_report(
        State(state): State<AppState>,
        ApiQuery(query): ApiQuery<WebhookDeliveryQuery>,
    ) -> Result<Response, AppError> {
        let filters = filter_from(Some(ReportFilterInput {
            start_date: query.start_date,
            end_date: query.end_date,
            user_id: None,
            status: query.status,
        }));
        let records = state
            .services
            .reporting
            .webhook_delivery_report(filters.as_ref())
            .await?;
        Ok(records_response("records", &records))
    }

    pub async fn get_system_metrics_report(
        State(state): State<AppState>,
        ApiQuery(_query): ApiQuery<SystemMetricsQuery>,
    ) -> Result<Response, AppError> {
        let metrics = state.services.reporting.system_metrics_report(None).await?;
        Ok(records_response("metrics", &metrics))
    }
}

fn records_response(key: &str, rows: &[ExportRow]) -> Response {
    let data = json!({ "recordCount": rows.len(), key: rows });
    (
        StatusCode::OK,
        Json(json!({ "isSuccessful": true, "data": data })),
    )
        .into_response()
}

fn csv_stream(
    rows: &[ExportRow],
) -> impl futures_util::Stream<Item = Result<Bytes, std::io::Error>> + use<> {
    let exporter = CsvExporter::new().with_bom(true);
    let headers: Vec<String> = rows
        .first()
        .map(|first| first.keys().cloned().collect())
        .unwrap_or_default();
    let mut chunks: Vec<Result<Bytes, std::io::Error>> = Vec::with_capacity(rows.len() + 1);
    if !headers.is_empty() {
        chunks.push(
            exporter
                .header_chunk(&headers)
                .map(Bytes::from)
                .map_err(std::io::Error::other),
        );
    }
    for row in rows {
        chunks.push(
            exporter
                .row_chunk(&headers, row)
                .map(Bytes::from)
                .map_err(std::io::Error::other),
        );
    }
    stream::iter(chunks)
}
