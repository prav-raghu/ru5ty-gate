use axum::Json;
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use ru5ty_gate_auth::refresh_ttl;
use ru5ty_gate_http::{
    AppError, AuthUser, ClientIp, ValidatedJson, clear_refresh_cookie_header,
    refresh_cookie_header, refresh_token_from_cookies,
};
use ru5ty_gate_types::ApiResponse;
use serde::Serialize;

use crate::dtos::LoginData;
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

fn with_cookie(mut response: Response, cookie: Option<HeaderValue>) -> Response {
    if let Some(value) = cookie {
        response.headers_mut().append(SET_COOKIE, value);
    }
    response
}

fn login_cookie(
    body: &ApiResponse<LoginData>,
    remember_me: bool,
    secure: bool,
) -> Option<HeaderValue> {
    let token = body.data.as_ref().map(|data| data.refresh_token.as_str())?;
    if token.is_empty() {
        return None;
    }
    refresh_cookie_header(token, refresh_ttl(remember_me), secure)
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

fn unauthorized_refresh(secure: bool) -> Response {
    with_cookie(
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::failure(
                "Invalid or expired refresh token",
            )),
        )
            .into_response(),
        clear_refresh_cookie_header(secure),
    )
}

pub struct AuthController;

impl AuthController {
    pub async fn login(
        State(state): State<AppState>,
        ClientIp(ip): ClientIp,
        ValidatedJson(body): ValidatedJson<LoginRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.login(&body, &ip).await?;
        let cookie = login_cookie(&result, body.remember_me, state.config.production);
        Ok(with_cookie(
            respond(StatusCode::OK, StatusCode::UNAUTHORIZED, result),
            cookie,
        ))
    }

    pub async fn verify_login_mfa(
        State(state): State<AppState>,
        ClientIp(ip): ClientIp,
        ValidatedJson(body): ValidatedJson<VerifyLoginMfaRequest>,
    ) -> Result<Response, AppError> {
        let result = state.services.auth.verify_login_mfa(&body, &ip).await?;
        let cookie = login_cookie(&result, body.remember_me, state.config.production);
        Ok(with_cookie(
            respond(StatusCode::OK, StatusCode::UNAUTHORIZED, result),
            cookie,
        ))
    }

    pub async fn refresh(
        State(state): State<AppState>,
        headers: HeaderMap,
        ValidatedJson(body): ValidatedJson<RefreshTokenRequest>,
    ) -> Result<Response, AppError> {
        let secure = state.config.production;
        let Some(token) = body
            .refresh_token
            .clone()
            .filter(|value| !value.is_empty())
            .or_else(|| refresh_token_from_cookies(&headers))
        else {
            return Ok(unauthorized_refresh(secure));
        };
        match state
            .services
            .auth
            .refresh_token(&token, body.remember_me)
            .await
        {
            Some(tokens) => {
                let cookie = refresh_cookie_header(
                    &tokens.refresh_token,
                    refresh_ttl(body.remember_me),
                    secure,
                );
                Ok(with_cookie(
                    (StatusCode::OK, Json(ApiResponse::success(tokens))).into_response(),
                    cookie,
                ))
            }
            None => Ok(unauthorized_refresh(secure)),
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
        Ok(with_cookie(
            (StatusCode::OK, Json(result)).into_response(),
            clear_refresh_cookie_header(state.config.production),
        ))
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
