use axum::Router;
use axum::middleware::from_fn_with_state;
use ru5ty_gate_http::authenticate;

use crate::plugins::auth_guard::build_authenticator;
use crate::routes::{
    AuthRoutes, BatchRoutes, DeviceRoutes, HealthRoutes, ReportingRoutes, UserRoutes, VenueRoutes,
};
use crate::types::AppState;

pub struct V1Routes;

impl V1Routes {
    pub fn register(state: &AppState) -> Router<AppState> {
        let protected = Router::new()
            .merge(AuthRoutes::protected())
            .merge(UserRoutes::protected())
            .merge(BatchRoutes::protected())
            .merge(ReportingRoutes::protected())
            .merge(VenueRoutes::protected())
            .route_layer(from_fn_with_state(build_authenticator(state), authenticate));
        Router::new()
            .merge(HealthRoutes::public())
            .merge(AuthRoutes::public())
            .merge(DeviceRoutes::register(state))
            .merge(protected)
    }
}
