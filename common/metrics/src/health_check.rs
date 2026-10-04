use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::health_status::HealthCheckResult;

pub type HealthCheckFuture = Pin<Box<dyn Future<Output = HealthCheckResult> + Send>>;

#[derive(Clone)]
pub struct HealthCheckDefinition {
    pub name: String,
    pub critical: bool,
    pub check: Arc<dyn Fn() -> HealthCheckFuture + Send + Sync>,
}
