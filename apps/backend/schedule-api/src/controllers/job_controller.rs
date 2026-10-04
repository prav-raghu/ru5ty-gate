use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, AppError};
use ru5ty_gate_types::ApiResponse;
use serde::Deserialize;

use crate::services::SchedulerError;
use crate::types::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct JobPath {
    pub name: String,
}

pub struct JobController;

impl JobController {
    pub async fn list_jobs(State(state): State<AppState>) -> Response {
        (
            StatusCode::OK,
            Json(ApiResponse::success(state.scheduler.statuses())),
        )
            .into_response()
    }

    pub async fn run_job(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<JobPath>,
    ) -> Result<Response, AppError> {
        match state.scheduler.run_now(&path.name).await {
            Ok(()) => Ok((
                StatusCode::OK,
                Json(ApiResponse::<()>::success_with_message((), "Job executed")),
            )
                .into_response()),
            Err(SchedulerError::JobNotFound(name)) => {
                Err(AppError::NotFound(format!("Job not found: {name}")))
            }
            Err(SchedulerError::JobFailed(reason)) => Err(AppError::Internal(reason)),
        }
    }
}
