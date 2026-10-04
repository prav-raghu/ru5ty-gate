use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};

use crate::controllers::DeviceController;
use crate::guards::device_key_guard;
use crate::types::AppState;

pub struct DeviceRoutes;

impl DeviceRoutes {
    pub fn register(state: &AppState) -> Router<AppState> {
        Router::new()
            .route(
                "/venues/{venueRef}/sessions/validate",
                post(DeviceController::validate_session),
            )
            .route("/venues/{venueRef}/policy", get(DeviceController::policy))
            .route(
                "/venues/{venueRef}/heartbeat",
                post(DeviceController::heartbeat),
            )
            .route("/venues/{venueRef}/sync", post(DeviceController::sync))
            .route_layer(from_fn_with_state(
                state.services.gateway.clone(),
                device_key_guard,
            ))
    }
}
