use ru5ty_gate_metrics::{MetricsError, MetricsRegistry};

const PREFIX: &str = "gateway_";
const SERVICE_NAME: &str = "api-gateway";
const IGNORED_PATHS: [&str; 7] = [
    "/metrics",
    "/health",
    "/health/live",
    "/health/ready",
    "/health/services",
    "/favicon.ico",
    "/docs",
];

pub fn install_metrics() -> Result<MetricsRegistry, MetricsError> {
    MetricsRegistry::install(PREFIX, SERVICE_NAME, &IGNORED_PATHS)
}
