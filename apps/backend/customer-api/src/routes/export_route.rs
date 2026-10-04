use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use ru5ty_gate_http::require_permission;
use ru5ty_gate_types::Permission;

use crate::controllers::ExportController;
use crate::types::AppState;

pub struct ExportRoutes;

impl ExportRoutes {
    pub fn protected() -> Router<AppState> {
        Router::new()
            .route("/users/export", get(ExportController::export_users_buffer))
            .route(
                "/users/export/stream",
                get(ExportController::export_users_stream),
            )
            .route_layer(from_fn_with_state(
                Permission::ReportExport,
                require_permission,
            ))
    }
}
