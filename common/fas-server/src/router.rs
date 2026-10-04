use axum::Router;
use axum::routing::get;

use crate::AppState;
use crate::controllers::{FasController, HealthController, StatusController};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/fas", get(FasController::auth))
        .route("/health", get(HealthController::health))
        .route("/status", get(StatusController::status))
        .with_state(state)
}
