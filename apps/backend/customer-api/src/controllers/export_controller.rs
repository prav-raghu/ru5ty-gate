use std::io;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use ru5ty_gate_export::{CsvExporter, ExcelExporter, ExportRow};
use ru5ty_gate_http::{ApiQuery, AppError};
use serde_json::Value;

use crate::dtos::ExportUser;
use crate::schemas::{ExportFormat, ExportQuery};
use crate::services::UserService;
use crate::types::AppState;

const BUFFER_LIMIT: i64 = 1000;
const PAGE_SIZE: i64 = 100;
const SPREADSHEET_LIMIT: i64 = 100_000;
const HEADERS: [&str; 4] = ["id", "email", "username", "createdAt"];

fn headers() -> Vec<String> {
    HEADERS.iter().map(|header| (*header).to_owned()).collect()
}

fn to_row(user: &ExportUser) -> ExportRow {
    let mut row = ExportRow::new();
    row.insert("id".to_owned(), Value::String(user.id.to_string()));
    row.insert("email".to_owned(), Value::String(user.email.clone()));
    row.insert("username".to_owned(), Value::String(user.username.clone()));
    row.insert(
        "createdAt".to_owned(),
        Value::String(user.created_at.to_rfc3339()),
    );
    row
}

fn attachment(format: ExportFormat, name: &str, body: Body) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    if let Ok(value) = header::HeaderValue::from_str(format.content_type()) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    if let Ok(value) = header::HeaderValue::from_str(&format!(
        "attachment; filename=\"{name}{}\"",
        format.extension()
    )) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

fn encode(format: ExportFormat, rows: &[ExportRow]) -> Result<Vec<u8>, AppError> {
    match format {
        ExportFormat::Csv => CsvExporter::new()
            .with_headers(headers())
            .with_bom(true)
            .export_to_bytes(rows),
        ExportFormat::Excel => ExcelExporter::new()
            .with_sheet_name("Users")
            .with_headers(headers())
            .export_to_bytes(rows),
    }
    .map_err(AppError::internal)
}

struct ExportCursor {
    users: UserService,
    exporter: CsvExporter,
    headers: Vec<String>,
    offset: i64,
    header_sent: bool,
    done: bool,
}

pub struct ExportController;

impl ExportController {
    pub async fn export_users_buffer(
        State(state): State<AppState>,
        ApiQuery(query): ApiQuery<ExportQuery>,
    ) -> Result<Response, AppError> {
        let users = state.services.user.export_users(0, BUFFER_LIMIT).await?;
        let rows: Vec<ExportRow> = users.iter().map(to_row).collect();
        let bytes = encode(query.format, &rows)?;
        Ok(attachment(query.format, "users", Body::from(bytes)))
    }

    pub async fn export_users_stream(
        State(state): State<AppState>,
        ApiQuery(query): ApiQuery<ExportQuery>,
    ) -> Result<Response, AppError> {
        if query.format == ExportFormat::Excel {
            let mut rows = Vec::new();
            let mut offset = 0;
            while offset < SPREADSHEET_LIMIT {
                let page = state.services.user.export_users(offset, PAGE_SIZE).await?;
                if page.is_empty() {
                    break;
                }
                offset += i64::try_from(page.len()).unwrap_or(PAGE_SIZE);
                rows.extend(page.iter().map(to_row));
            }
            let bytes = encode(ExportFormat::Excel, &rows)?;
            return Ok(attachment(
                ExportFormat::Excel,
                "users-export",
                Body::from(bytes),
            ));
        }

        let cursor = ExportCursor {
            users: state.services.user.clone(),
            exporter: CsvExporter::new().with_bom(true),
            headers: headers(),
            offset: 0,
            header_sent: false,
            done: false,
        };
        let body = stream::unfold(cursor, |mut cursor| async move {
            if cursor.done {
                return None;
            }
            if !cursor.header_sent {
                cursor.header_sent = true;
                let chunk = cursor
                    .exporter
                    .header_chunk(&cursor.headers)
                    .map(Bytes::from)
                    .map_err(io::Error::other);
                return Some((chunk, cursor));
            }
            match cursor.users.export_users(cursor.offset, PAGE_SIZE).await {
                Ok(page) if page.is_empty() => None,
                Ok(page) => {
                    cursor.offset += i64::try_from(page.len()).unwrap_or(PAGE_SIZE);
                    let mut buffer = Vec::new();
                    for user in &page {
                        match cursor.exporter.row_chunk(&cursor.headers, &to_row(user)) {
                            Ok(chunk) => buffer.extend(chunk),
                            Err(error) => {
                                cursor.done = true;
                                return Some((Err(io::Error::other(error)), cursor));
                            }
                        }
                    }
                    Some((Ok(Bytes::from(buffer)), cursor))
                }
                Err(error) => {
                    cursor.done = true;
                    Some((Err(io::Error::other(error.to_string())), cursor))
                }
            }
        });
        Ok(attachment(
            ExportFormat::Csv,
            "users-export",
            Body::from_stream(body),
        ))
    }
}
