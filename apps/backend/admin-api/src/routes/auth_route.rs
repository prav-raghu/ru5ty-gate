use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use ru5ty_gate_http::{RateLimiter, rate_limit};

use crate::config::RateLimitConfig;
use crate::controllers::AuthController;
use crate::types::AppState;

pub struct AuthRoutes;

impl AuthRoutes {
    pub fn public() -> Router<AppState> {
        let tier = || from_fn_with_state(RateLimiter::new(RateLimitConfig::auth()), rate_limit);
        Router::new()
            .route(
                "/auth/login",
                post(AuthController::login).route_layer(tier()),
            )
            .route(
                "/auth/verify-login-mfa",
                post(AuthController::verify_login_mfa).route_layer(tier()),
            )
            .route(
                "/auth/refresh",
                post(AuthController::refresh).route_layer(tier()),
            )
            .route(
                "/auth/forgot-password",
                post(AuthController::forgot_password).route_layer(tier()),
            )
            .route(
                "/auth/reset-password",
                post(AuthController::reset_password).route_layer(tier()),
            )
            .route(
                "/auth/bootstrap-admin",
                post(AuthController::bootstrap_admin).route_layer(tier()),
            )
    }

    pub fn protected() -> Router<AppState> {
        Router::new()
            .route("/auth/logout", get(AuthController::log_out))
            .route("/auth/me", get(AuthController::get_current_user))
    }
}
