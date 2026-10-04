use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::post;
use ru5ty_gate_http::require_permission;
use ru5ty_gate_types::Permission;

use crate::controllers::BatchController;
use crate::types::AppState;

pub struct BatchRoutes;

impl BatchRoutes {
    pub fn protected() -> Router<AppState> {
        Router::new()
            .route(
                "/batch/users/create",
                post(BatchController::bulk_create_users),
            )
            .route(
                "/batch/users/update-status",
                post(BatchController::bulk_update_user_status),
            )
            .route(
                "/batch/users/delete",
                post(BatchController::bulk_delete_users),
            )
            .route("/batch/custom", post(BatchController::execute_custom_batch))
            .route_layer(from_fn_with_state(
                Permission::BatchWrite,
                require_permission,
            ))
    }
}
