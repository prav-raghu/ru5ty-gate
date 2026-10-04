use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_observability::capture_error;
use ru5ty_gate_types::{ApiResponse, FieldError};
use serde_json::json;

#[derive(Debug, Clone)]
pub enum AppError {
    BadRequest(String),
    Validation(Vec<FieldError>),
    Unauthorized,
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    TooManyRequests {
        message: String,
        retry_after_seconds: u64,
    },
    Internal(String),
}

impl AppError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Internal(error.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message)
            | Self::Forbidden(message)
            | Self::NotFound(message)
            | Self::Conflict(message)
            | Self::Internal(message) => formatter.write_str(message),
            Self::Validation(_) => formatter.write_str("Validation failed"),
            Self::Unauthorized => formatter.write_str("Unauthorized"),
            Self::TooManyRequests { message, .. } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        match self {
            Self::Validation(errors) => {
                (status, Json(ApiResponse::<()>::validation_failure(errors))).into_response()
            }
            Self::Unauthorized => {
                (status, Json(ApiResponse::<()>::failure("Unauthorized"))).into_response()
            }
            Self::Internal(message) => {
                tracing::error!(error = %message, "unhandled request error");
                capture_error(&Self::Internal(message));
                (
                    status,
                    Json(ApiResponse::<()>::failure("Internal server error")),
                )
                    .into_response()
            }
            Self::TooManyRequests {
                message,
                retry_after_seconds,
            } => {
                let body = json!({
                    "isSuccessful": false,
                    "statusCode": 429,
                    "error": "Too Many Requests",
                    "message": message,
                });
                let mut response = (status, Json(body)).into_response();
                if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
                    response.headers_mut().insert(header::RETRY_AFTER, value);
                }
                response
            }
            Self::BadRequest(message)
            | Self::Forbidden(message)
            | Self::NotFound(message)
            | Self::Conflict(message) => {
                (status, Json(ApiResponse::<()>::failure(message))).into_response()
            }
        }
    }
}
