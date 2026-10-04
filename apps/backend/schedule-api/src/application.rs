use std::sync::Arc;

use axum::middleware::{from_fn, from_fn_with_state};
use axum::{Extension, Router};
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_database::{DatabaseError, PgPool};
use ru5ty_gate_http::{
    RateLimiter, ServerInfo, TrustedProxyHops, api_version, catch_panic_layer, cors_layer,
    not_found, rate_limit, request_logger, response_timestamp, security_headers, serve,
};
use thiserror::Error;
use tower::ServiceBuilder;

use crate::config::{RateLimitConfig, ServiceConfig};
use crate::plugins::database::connect_database;
use crate::plugins::services::build_scheduler;
use crate::routes::v1::V1Routes;
use crate::routes::v2::V2Routes;
use crate::services::CronSchedulerService;
use crate::types::AppState;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

pub struct Application {
    state: AppState,
}

impl Application {
    pub async fn initialize(config: ServiceConfig) -> Result<Self, ApplicationError> {
        let pool = connect_database(&config).await?;
        let redis =
            RedisService::connect(&config.redis_url, config.redis_tls_reject_unauthorized).await;
        Ok(Self::with_parts(config, pool, redis))
    }

    pub fn with_parts(config: ServiceConfig, pool: PgPool, redis: RedisService) -> Self {
        let scheduler: Arc<CronSchedulerService> = Arc::new(build_scheduler(&config, &pool));
        Self {
            state: AppState {
                config: Arc::new(config),
                pool,
                redis,
                scheduler,
            },
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn router(&self) -> Router {
        let global_limiter = RateLimiter::new(RateLimitConfig::global());
        let middleware = ServiceBuilder::new()
            .layer(Extension(TrustedProxyHops(
                self.state.config.trusted_proxy_hops,
            )))
            .layer(catch_panic_layer())
            .layer(cors_layer(&self.state.config.cors_origin))
            .layer(from_fn(security_headers))
            .layer(from_fn_with_state(global_limiter, rate_limit))
            .layer(from_fn(request_logger))
            .layer(from_fn(api_version))
            .layer(from_fn(response_timestamp));
        Router::new()
            .nest("/api/v1", V1Routes::register(&self.state))
            .nest("/api/v2", V2Routes::register())
            .fallback(not_found)
            .layer(middleware)
            .with_state(self.state.clone())
    }

    pub async fn start(self) -> std::io::Result<()> {
        let info = ServerInfo::new(
            "schedule-api",
            env!("CARGO_PKG_VERSION"),
            self.state.config.port,
            self.state.config.production,
        );
        let scheduler = self.state.scheduler.clone();
        scheduler.start_all();
        let result = serve(self.router(), &info).await;
        scheduler.stop_all().await;
        result
    }
}
