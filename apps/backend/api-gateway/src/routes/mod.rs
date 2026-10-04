mod graphql_route;
mod health_route;
mod metrics_route;
mod proxy_route;

pub use graphql_route::GraphqlRoutes;
pub use health_route::HealthRoutes;
pub use metrics_route::MetricsRoutes;
pub use proxy_route::ProxyRoutes;
