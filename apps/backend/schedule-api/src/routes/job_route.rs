use axum::Router;
use axum::routing::{get, post};

use crate::controllers::JobController;
use crate::types::AppState;

pub struct JobRoutes;

impl JobRoutes {
    pub fn protected() -> Router<AppState> {
        Router::new()
            .route("/jobs", get(JobController::list_jobs))
            .route("/jobs/{name}/run", post(JobController::run_job))
    }
}
