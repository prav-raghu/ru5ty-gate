use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post, put};
use ru5ty_gate_http::require_permission;
use ru5ty_gate_types::Permission;

use crate::controllers::{CaptiveSessionController, GatewayController, VenueController};
use crate::types::AppState;

pub struct VenueRoutes;

impl VenueRoutes {
    pub fn protected() -> Router<AppState> {
        let read = || from_fn_with_state(Permission::VenueRead, require_permission);
        let write = || from_fn_with_state(Permission::VenueWrite, require_permission);

        Router::new()
            .route(
                "/venues",
                get(VenueController::list_venues)
                    .route_layer(read())
                    .merge(post(VenueController::create_venue).route_layer(write())),
            )
            .route(
                "/venues/{venueRef}",
                get(VenueController::get_venue)
                    .route_layer(read())
                    .merge(put(VenueController::update_venue).route_layer(write())),
            )
            .route(
                "/venues/{venueRef}/gateways",
                get(GatewayController::list_gateways)
                    .route_layer(read())
                    .merge(post(GatewayController::create_gateway).route_layer(write())),
            )
            .route(
                "/venues/{venueRef}/sessions",
                get(CaptiveSessionController::list_sessions).route_layer(read()),
            )
            .route(
                "/gateways/{gatewayId}/rotate-key",
                post(GatewayController::rotate_key).route_layer(write()),
            )
    }
}
