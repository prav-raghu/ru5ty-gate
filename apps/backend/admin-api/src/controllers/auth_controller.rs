use axum::Json;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{AppError, AuthUser, ClientIp, ValidatedJson};
use ru5ty_gate_types::ApiResponse;
use serde::Serialize;

use crate::schemas::{
    BootstrapAdminRequest, ForgotPasswordRequest, LoginRequest, RefreshTokenRequest,
    ResetPasswordRequest, VerifyLoginMfaRequest,
};
use crate::types::AppState;

fn respond<T: Serialize>(
    success: StatusCode,
    failure: StatusCode,
    body: ApiResponse<T>,
) -> Response {
    let status = if body.is_successful { success } else { failure };
    (status, Json(body)).into_response()
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

pub struct AuthController;

impl AuthController {
    pub async fn login(
        State(state): State<AppState>,
        ClientIp(ip): ClientIp,
        ValidatedJson(body): ValidatedJson<LoginRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.login(&body, &ip).await?;
        Ok(respond(StatusCode::OK, StatusCode::UNAUTHORIZED, result))
    }

    pub async fn verify_login_mfa(
        State(state): State<AppState>,
        ClientIp(ip): ClientIp,
        ValidatedJson(body): ValidatedJson<VerifyLoginMfaRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.verify_login_mfa(&body, &ip).await?;
        Ok(respond(StatusCode::OK, StatusCode::UNAUTHORIZED, result))
    }

    pub async fn refresh(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<RefreshTokenRequest>,
    ) -> Result<Response, AppError> {
        match state
            .services
            .auth
            .refresh_token(&body.refresh_token, body.remember_me)
            .await
        {
            Some(tokens) => {
                Ok((StatusCode::OK, Json(ApiResponse::success(tokens))).into_response())
            }
            None => Ok((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<()>::failure(
                    "Invalid or expired refresh token",
                )),
            )
                .into_response()),
        }
    }

    pub async fn log_out(
        State(state): State<AppState>,
        user: AuthUser,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .auth
            .logout(user.id, bearer(&headers))
            .await?;
        Ok((StatusCode::OK, Json(result)).into_response())
    }

    pub async fn forgot_password(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<ForgotPasswordRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.forgot_password(&body.email).await?;
        Ok((StatusCode::OK, Json(result)).into_response())
    }

    pub async fn reset_password(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<ResetPasswordRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.reset_password(&body).await?;
        Ok(respond(StatusCode::OK, StatusCode::BAD_REQUEST, result))
    }

    pub async fn get_current_user(
        State(state): State<AppState>,
        user: AuthUser,
    ) -> Result<Response, AppError> {
        match state.services.auth.get_current_user(user.id).await? {
            Some(current) => Ok((StatusCode::OK, Json(current)).into_response()),
            None => Err(AppError::Unauthorized),
        }
    }

    pub async fn bootstrap_admin(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<BootstrapAdminRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.bootstrap.bootstrap_admin(&body).await?;
        Ok(respond(StatusCode::CREATED, StatusCode::FORBIDDEN, result))
    }
}
