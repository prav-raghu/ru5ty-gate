use axum::Router;
use axum::routing::get;

use crate::controllers::HealthController;
use crate::types::AppState;

pub struct HealthRoutes;

impl HealthRoutes {
    pub fn public() -> Router<AppState> {
        Router::new()
            .route("/ping", get(HealthController::check_health))
            .route("/ready", get(HealthController::get_readiness))
    }
}
