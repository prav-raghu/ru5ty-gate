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
        let auth_tier =
            || from_fn_with_state(RateLimiter::new(RateLimitConfig::auth()), rate_limit);
        let sensitive_tier = from_fn_with_state(
            RateLimiter::new(RateLimitConfig::sensitive_endpoints()),
            rate_limit,
        );
        Router::new()
            .route(
                "/auth/register",
                post(AuthController::register).route_layer(auth_tier()),
            )
            .route(
                "/auth/login",
                post(AuthController::login).route_layer(auth_tier()),
            )
            .route(
                "/auth/refresh",
                post(AuthController::refresh).route_layer(auth_tier()),
            )
            .route("/auth/verify/{token}", get(AuthController::verify_email))
            .route(
                "/auth/resend-verification-email",
                post(AuthController::resend_verification_email).route_layer(sensitive_tier),
            )
    }

    pub fn protected() -> Router<AppState> {
        Router::new().route("/auth/logout", post(AuthController::logout))
    }
}
