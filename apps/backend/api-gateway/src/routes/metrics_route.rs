use axum::Router;
use axum::routing::get;
use ru5ty_gate_metrics::{MetricsRegistry, render_metrics};

pub struct MetricsRoutes;

impl MetricsRoutes {
    pub fn register(registry: MetricsRegistry) -> Router {
        Router::new()
            .route("/metrics", get(render_metrics))
            .with_state(registry)
    }
}
