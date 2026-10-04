use axum::Router;
use axum::routing::get;
use ru5ty_gate_metrics::{HealthState, health_router};

use crate::controllers::HealthController;
use crate::types::AppState;

pub struct HealthRoutes;

impl HealthRoutes {
    pub fn register(state: AppState, health_state: HealthState) -> Router {
        let services = Router::new()
            .route("/health/services", get(HealthController::services))
            .route("/health/services/{name}", get(HealthController::service))
            .with_state(state);
        health_router(health_state).merge(services)
    }
}
