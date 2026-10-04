use std::sync::Arc;

use ru5ty_gate_cache::RedisService;
use ru5ty_gate_database::PgPool;

use crate::config::ServiceConfig;
use crate::types::services::Services;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServiceConfig>,
    pub pool: PgPool,
    pub redis: RedisService,
    pub services: Arc<Services>,
}
