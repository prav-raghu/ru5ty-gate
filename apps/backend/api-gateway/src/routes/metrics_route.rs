use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use ru5ty_gate_metrics::{MetricsRegistry, render_metrics};

use crate::guards::{MetricsToken, metrics_guard};

pub struct MetricsRoutes;

impl MetricsRoutes {
    pub fn register(registry: MetricsRegistry, token: MetricsToken) -> Router {
        Router::new()
            .route("/metrics", get(render_metrics))
            .route_layer(from_fn_with_state(token, metrics_guard))
            .with_state(registry)
    }
}
