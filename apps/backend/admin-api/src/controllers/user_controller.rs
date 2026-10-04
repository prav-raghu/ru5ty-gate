use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, AppError, AuthUser, ValidatedJson};
use ru5ty_gate_types::{ApiResponse, FieldError};
use serde::Serialize;
use validator::ValidateEmail;

use crate::schemas::{
    ChangePasswordRequest, Disable2FaRequest, EmailPath, OnboardingRequest,
    ResendVerificationRequest, UpdateProfileRequest, UserIdPath, UsernamePath, Verify2FaRequest,
};
use crate::types::AppState;

fn ok_or_bad_request<T: Serialize>(body: ApiResponse<T>) -> Response {
    let status = if body.is_successful {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(body)).into_response()
}

pub struct UserController;

impl UserController {
    pub async fn onboard_user(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<OnboardingRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.user.onboard_user(&body, &user).await?;
        Ok(ok_or_bad_request(result))
    }

    pub async fn resend_verification_email(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<ResendVerificationRequest>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .user
            .resend_verification_email(&body.email)
            .await?;
        Ok(ok_or_bad_request(result))
    }

    pub async fn get_user_roles(State(state): State<AppState>) -> Result<Response, AppError> {
        Ok(ok_or_bad_request(
            state.services.user.get_user_roles().await?,
        ))
    }

    pub async fn get_user_statuses(State(state): State<AppState>) -> Result<Response, AppError> {
        Ok(ok_or_bad_request(
            state.services.user.get_user_statuses().await?,
        ))
    }

    pub async fn check_username_availability(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<UsernamePath>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .user
            .is_username_available(&path.username)
            .await?;
        Ok((StatusCode::OK, Json(result)).into_response())
    }

    pub async fn check_email_availability(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<EmailPath>,
    ) -> Result<Response, AppError> {
        if !path.email.validate_email() {
            return Err(AppError::Validation(vec![FieldError {
                field: "email".to_owned(),
                message: "Must be a valid email address".to_owned(),
            }]));
        }
        let result = state.services.user.is_email_available(&path.email).await?;
        Ok((StatusCode::OK, Json(result)).into_response())
    }

    pub async fn update_profile(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<UpdateProfileRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.user.update_profile(user.id, &body).await?;
        Ok(ok_or_bad_request(result))
    }

    pub async fn change_password(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<ChangePasswordRequest>,
    ) -> Result<Response, AppError> {
        if body.new_password != body.confirm_password {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::failure("Passwords do not match")),
            )
                .into_response());
        }
        let result = state
            .services
            .user
            .change_password(user.id, &body.current_password, &body.new_password)
            .await?;
        Ok(ok_or_bad_request(result))
    }

    pub async fn setup_2fa(
        State(state): State<AppState>,
        user: AuthUser,
    ) -> Result<Response, AppError> {
        Ok(ok_or_bad_request(
            state.services.user.setup_2fa(user.id).await?,
        ))
    }

    pub async fn verify_2fa(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<Verify2FaRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.user.verify_2fa(user.id, &body.token).await?;
        Ok(ok_or_bad_request(result))
    }

    pub async fn disable_2fa(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<Disable2FaRequest>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .user
            .disable_2fa(user.id, &body.token)
            .await?;
        Ok(ok_or_bad_request(result))
    }

    pub async fn get_user_details(
        State(state): State<AppState>,
        ApiPath(path): ApiPath<UserIdPath>,
    ) -> Result<Response, AppError> {
        match state.services.user.get_user_details(path.user_id).await? {
            Some(details) => Ok((StatusCode::OK, Json(details)).into_response()),
            None => Err(AppError::NotFound("User not found".to_owned())),
        }
    }
}
