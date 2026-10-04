use std::sync::Arc;

use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};
use ru5ty_gate_http::{
    RateLimitPolicy, RateLimiter, ServerInfo, catch_panic_layer, cors_layer, not_found, rate_limit,
    request_logger, security_headers, serve,
};
use ru5ty_gate_metrics::{MetricsError, MetricsRegistry, track_http};
use thiserror::Error;
use tower::ServiceBuilder;

use crate::config::{ProxyConfig, ServiceConfig};
use crate::plugins::graphql::graphql_router;
use crate::plugins::health::{health_state, service_endpoints};
use crate::plugins::metrics::install_metrics;
use crate::routes::{HealthRoutes, MetricsRoutes, ProxyRoutes};
use crate::services::HealthService;
use crate::types::AppState;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Metrics(#[from] MetricsError),
}

pub struct Application {
    state: AppState,
    metrics: Option<MetricsRegistry>,
}

impl Application {
    pub fn initialize(config: ServiceConfig) -> Result<Self, ApplicationError> {
        let metrics = install_metrics()?;
        Ok(Self::with_parts(config, Some(metrics)))
    }

    pub fn with_parts(config: ServiceConfig, metrics: Option<MetricsRegistry>) -> Self {
        let health = Arc::new(HealthService::new(service_endpoints(&config)));
        Self {
            state: AppState {
                config: Arc::new(config),
                health,
            },
            metrics,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn router(&self) -> Router {
        let config = &self.state.config;
        let limiter = RateLimiter::new(RateLimitPolicy::per_minute(
            config.rate_limit_max,
            "Rate limit exceeded, retry in 1 minute",
        ));

        let mut router = Router::new()
            .merge(HealthRoutes::register(
                self.state.clone(),
                health_state(&self.state.health),
            ))
            .merge(ProxyRoutes::register(ProxyConfig::targets(config)));
        if let Some(graphql) = graphql_router(config) {
            router = router.merge(graphql);
        }
        if let Some(metrics) = &self.metrics {
            router = router
                .merge(MetricsRoutes::register(metrics.clone()))
                .layer(from_fn_with_state(metrics.clone(), track_http));
        }

        let middleware = ServiceBuilder::new()
            .layer(catch_panic_layer())
            .layer(cors_layer(&config.cors_origin))
            .layer(from_fn(security_headers))
            .layer(from_fn_with_state(limiter, rate_limit))
            .layer(from_fn(request_logger));
        router.fallback(not_found).layer(middleware)
    }

    pub async fn start(self) -> std::io::Result<()> {
        let info = ServerInfo::new(
            "api-gateway",
            env!("CARGO_PKG_VERSION"),
            self.state.config.port,
            self.state.config.production,
        );
        serve(self.router(), &info).await
    }
}
