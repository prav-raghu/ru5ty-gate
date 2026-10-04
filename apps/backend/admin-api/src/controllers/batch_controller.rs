use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{AppError, AuthUser, ValidatedJson};
use ru5ty_gate_types::{ApiResponse, BatchOperationSummary};

use crate::schemas::{
    BulkCreateUsersRequest, BulkDeleteUsersRequest, BulkUpdateStatusRequest, CustomBatchRequest,
};
use crate::types::AppState;

fn summary_response(summary: BatchOperationSummary) -> Response {
    let status = if summary.failed > 0 {
        StatusCode::MULTI_STATUS
    } else {
        StatusCode::OK
    };
    let mut body = ApiResponse::success(summary);
    body.is_successful = body.data.as_ref().is_some_and(|data| data.failed == 0);
    (status, Json(body)).into_response()
}

pub struct BatchController;

impl BatchController {
    pub async fn bulk_create_users(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<BulkCreateUsersRequest>,
    ) -> Result<Response, AppError> {
        let summary = state
            .services
            .batch
            .bulk_create_users(&body.users, user.id)
            .await?;
        Ok(summary_response(summary))
    }

    pub async fn bulk_update_user_status(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<BulkUpdateStatusRequest>,
    ) -> Result<Response, AppError> {
        let summary = state
            .services
            .batch
            .bulk_update_user_status(&body.updates, user.id)
            .await?;
        Ok(summary_response(summary))
    }

    pub async fn bulk_delete_users(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<BulkDeleteUsersRequest>,
    ) -> Result<Response, AppError> {
        let summary = state
            .services
            .batch
            .bulk_delete_users(&body.user_ids, user.id)
            .await?;
        Ok(summary_response(summary))
    }

    pub async fn execute_custom_batch(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<CustomBatchRequest>,
    ) -> Result<Response, AppError> {
        Ok(summary_response(
            state.services.batch.execute_custom_batch(&body.items),
        ))
    }
}
