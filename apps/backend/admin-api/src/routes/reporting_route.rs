use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use ru5ty_gate_http::require_permission;
use ru5ty_gate_types::Permission;

use crate::controllers::ReportingController;
use crate::types::AppState;

pub struct ReportingRoutes;

impl ReportingRoutes {
    pub fn protected() -> Router<AppState> {
        let exports = Router::new()
            .route(
                "/reports/generate",
                post(ReportingController::generate_report),
            )
            .route("/reports/stream", get(ReportingController::stream_report))
            .route_layer(from_fn_with_state(
                Permission::ReportExport,
                require_permission,
            ));
        let views = Router::new()
            .route(
                "/reports/user-activity",
                get(ReportingController::get_user_activity_report),
            )
            .route(
                "/reports/webhook-delivery",
                get(ReportingController::get_webhook_delivery_report),
            )
            .route(
                "/reports/system-metrics",
                get(ReportingController::get_system_metrics_report),
            )
            .route_layer(from_fn_with_state(
                Permission::ReportView,
                require_permission,
            ));
        Router::new().merge(exports).merge(views)
    }
}
