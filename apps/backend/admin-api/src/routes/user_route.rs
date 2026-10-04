use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post, put};
use ru5ty_gate_http::{RateLimiter, rate_limit, require_permission};
use ru5ty_gate_types::Permission;

use crate::config::RateLimitConfig;
use crate::controllers::UserController;
use crate::types::AppState;

pub struct UserRoutes;

impl UserRoutes {
    pub fn protected() -> Router<AppState> {
        let sensitive = || {
            from_fn_with_state(
                RateLimiter::new(RateLimitConfig::sensitive_endpoints()),
                rate_limit,
            )
        };
        let permission = |required: Permission| from_fn_with_state(required, require_permission);

        let reads = Router::new()
            .route("/users/roles", get(UserController::get_user_roles))
            .route("/users/statuses", get(UserController::get_user_statuses))
            .route(
                "/users/check-email/{email}",
                get(UserController::check_email_availability).route_layer(sensitive()),
            )
            .route(
                "/users/check-username/{username}",
                get(UserController::check_username_availability).route_layer(sensitive()),
            )
            .route(
                "/users/{userId}/details",
                get(UserController::get_user_details).route_layer(from_fn_with_state(
                    RateLimiter::new(RateLimitConfig::admin_operations()),
                    rate_limit,
                )),
            )
            .route_layer(permission(Permission::UserRead));

        let writes = Router::new()
            .route("/users/onboarding", post(UserController::onboard_user))
            .route(
                "/users/resend-verification",
                post(UserController::resend_verification_email),
            )
            .route_layer(permission(Permission::UserWrite));

        let own_account = Router::new()
            .route("/users/profile", put(UserController::update_profile))
            .route(
                "/users/change-password",
                post(UserController::change_password).route_layer(sensitive()),
            )
            .route(
                "/users/2fa/setup",
                post(UserController::setup_2fa).route_layer(sensitive()),
            )
            .route(
                "/users/2fa/verify",
                post(UserController::verify_2fa).route_layer(sensitive()),
            )
            .route(
                "/users/2fa/disable",
                post(UserController::disable_2fa).route_layer(sensitive()),
            );

        Router::new().merge(reads).merge(writes).merge(own_account)
    }
}
