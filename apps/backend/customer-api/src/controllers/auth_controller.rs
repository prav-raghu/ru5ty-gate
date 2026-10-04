use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, AppError, AuthUser, ClientIp, ValidatedJson, parse_validated};
use ru5ty_gate_types::ApiResponse;
use serde::Serialize;

use crate::schemas::{
    LoginRequest, LogoutRequest, RefreshTokenRequest, RegisterRequest, ResendVerificationRequest,
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
}

pub struct AuthController;

impl AuthController {
    pub async fn register(
        State(state): State<AppState>,
        ClientIp(ip): ClientIp,
        ValidatedJson(body): ValidatedJson<RegisterRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.register(&body, &ip).await?;
        Ok(respond(
            StatusCode::CREATED,
            StatusCode::BAD_REQUEST,
            result,
        ))
    }

    pub async fn login(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<LoginRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.login(&body).await?;
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
            None => Err(AppError::Unauthorized),
        }
    }

    pub async fn logout(
        State(state): State<AppState>,
        user: AuthUser,
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<Response, AppError> {
        let request = if body.is_empty() {
            LogoutRequest::default()
        } else {
            parse_validated::<LogoutRequest>(&body)?
        };
        state
            .services
            .auth
            .logout(user.id, bearer(&headers), request.refresh_token.as_deref())
            .await?;
        Ok(StatusCode::NO_CONTENT.into_response())
    }

    pub async fn verify_email(
        State(state): State<AppState>,
        ApiPath(token): ApiPath<String>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.verify_email(&token).await?;
        Ok(respond(StatusCode::OK, StatusCode::BAD_REQUEST, result))
    }

    pub async fn resend_verification_email(
        State(state): State<AppState>,
        ValidatedJson(body): ValidatedJson<ResendVerificationRequest>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .auth
            .resend_verification_email(&body.email)
            .await?;
        Ok((StatusCode::OK, Json(result)).into_response())
    }
}
