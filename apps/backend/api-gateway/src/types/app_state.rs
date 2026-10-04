use std::sync::Arc;

use crate::config::ServiceConfig;
use crate::services::HealthService;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServiceConfig>,
    pub health: Arc<HealthService>,
}
