use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_metrics::HealthStatus;
use serde_json::json;

use crate::types::AppState;

fn status_code(status: HealthStatus) -> StatusCode {
    match status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::MULTI_STATUS,
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    }
}

pub struct HealthController;

impl HealthController {
    pub async fn services(State(state): State<AppState>) -> Response {
        let result = state.health.check_all_services().await;
        (status_code(result.status), Json(result)).into_response()
    }

    pub async fn service(State(state): State<AppState>, Path(name): Path<String>) -> Response {
        match state.health.check_service_by_name(&name).await {
            Some(result) => (status_code(result.status), Json(result)).into_response(),
            None => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Service not found" })),
            )
                .into_response(),
        }
    }
}
