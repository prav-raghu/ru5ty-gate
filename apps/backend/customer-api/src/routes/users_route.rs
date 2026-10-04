use axum::Router;
use axum::routing::get;

use crate::controllers::UsersController;
use crate::types::AppState;

pub struct UsersRoutes;

impl UsersRoutes {
    pub fn protected() -> Router<AppState> {
        Router::new()
            .route("/users", get(UsersController::get_users))
            .route("/users/{userId}", get(UsersController::get_user))
    }
}
