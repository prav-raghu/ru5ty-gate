use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use metrics::{counter, gauge, histogram};

use crate::metrics_registry::MetricsRegistry;

pub async fn track_http(
    State(registry): State<MetricsRegistry>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    if registry.is_ignored(&path) {
        return next.run(request).await;
    }

    let method = request.method().to_string();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| path.clone(), |matched| matched.as_str().to_owned());
    let active = registry.metric_name("http_active_connections_total");
    let started = Instant::now();

    gauge!(active.clone()).increment(1.0);
    let response = next.run(request).await;
    gauge!(active).decrement(1.0);

    let status = response.status().as_u16().to_string();
    let labels = [
        ("method", method),
        ("route", route),
        ("status_code", status),
    ];
    counter!(registry.metric_name("http_requests_total"), &labels).increment(1);
    histogram!(
        registry.metric_name("http_request_duration_seconds"),
        &labels
    )
    .record(started.elapsed().as_secs_f64());
    response
}
