use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::metrics_error::MetricsError;

#[derive(Clone)]
pub struct MetricsRegistry {
    handle: PrometheusHandle,
    prefix: Arc<str>,
    ignore_paths: Arc<[String]>,
}

impl MetricsRegistry {
    pub fn install(
        prefix: &str,
        service: &str,
        ignore_paths: &[&str],
    ) -> Result<Self, MetricsError> {
        let handle = PrometheusBuilder::new()
            .add_global_label("service", service)
            .install_recorder()
            .map_err(|_| MetricsError::RecorderAlreadyInstalled)?;
        Ok(Self {
            handle,
            prefix: Arc::from(prefix),
            ignore_paths: ignore_paths.iter().map(|path| (*path).to_owned()).collect(),
        })
    }

    pub fn metric_name(&self, name: &str) -> String {
        format!("{}{name}", self.prefix)
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        self.ignore_paths
            .iter()
            .any(|ignored| path == ignored || path.starts_with(&format!("{ignored}/")))
    }

    pub fn render(&self) -> String {
        self.handle.render()
    }
}

pub async fn render_metrics(State(registry): State<MetricsRegistry>) -> Response {
    let mut response = (StatusCode::OK, registry.render()).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    response
}
