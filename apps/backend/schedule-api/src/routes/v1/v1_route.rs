use axum::Router;
use axum::middleware::from_fn_with_state;

use crate::guards::{ApiKey, api_key_guard};
use crate::routes::{HealthRoutes, JobRoutes};
use crate::types::AppState;

pub struct V1Routes;

impl V1Routes {
    pub fn register(state: &AppState) -> Router<AppState> {
        let protected = JobRoutes::protected().route_layer(from_fn_with_state(
            ApiKey::new(&state.config.schedule_api_key),
            api_key_guard,
        ));
        Router::new().merge(HealthRoutes::public()).merge(protected)
    }
}
