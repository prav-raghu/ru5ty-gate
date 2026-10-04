use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, ApiQuery, AppError, AuthUser};
use ru5ty_gate_types::ApiResponse;

use crate::schemas::{UserListQuery, UserPath};
use crate::services::UserFilters;
use crate::types::AppState;

pub struct UsersController;

impl UsersController {
    pub async fn get_users(
        State(state): State<AppState>,
        user: AuthUser,
        ApiQuery(query): ApiQuery<UserListQuery>,
    ) -> Result<Response, AppError> {
        let filters = UserFilters {
            gender: query.gender,
            min_age: query.min_age.and_then(|value| i32::try_from(value).ok()),
            max_age: query.max_age.and_then(|value| i32::try_from(value).ok()),
            limit: query.limit.map(i64::from),
            offset: query.offset.map(i64::from),
        };
        let users = state
            .services
            .user
            .get_users(&filters, Some(user.id))
            .await?;
        Ok((StatusCode::OK, Json(ApiResponse::success(users))).into_response())
    }

    pub async fn get_user(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<UserPath>,
    ) -> Result<Response, AppError> {
        match state.services.user.get_user_by_id(path.user_id).await? {
            Some(user) => Ok((StatusCode::OK, Json(ApiResponse::success(user))).into_response()),
            None => Err(AppError::NotFound("User not found".to_owned())),
        }
    }
}
