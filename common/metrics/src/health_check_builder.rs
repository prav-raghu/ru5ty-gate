use std::future::Future;
use std::sync::Arc;

use crate::health_check::{HealthCheckDefinition, HealthCheckFuture};
use crate::health_status::{HealthCheckResult, HealthStatus};

#[derive(Default)]
#[must_use]
pub struct HealthCheckBuilder {
    checks: Vec<HealthCheckDefinition>,
}

fn boxed_check<F, Fut>(
    probe: F,
    healthy: String,
    unhealthy: String,
    failing: HealthStatus,
) -> Arc<dyn Fn() -> HealthCheckFuture + Send + Sync>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = bool> + Send + 'static,
{
    Arc::new(move || {
        let outcome = probe();
        let healthy = healthy.clone();
        let unhealthy = unhealthy.clone();
        Box::pin(async move {
            if outcome.await {
                HealthCheckResult::new(HealthStatus::Healthy, healthy)
            } else {
                HealthCheckResult::new(failing, unhealthy)
            }
        })
    })
}

impl HealthCheckBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_database_check<F, Fut>(mut self, probe: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        self.checks.push(HealthCheckDefinition {
            name: "database".to_owned(),
            critical: true,
            check: boxed_check(
                probe,
                "Database connection is healthy".to_owned(),
                "Database connection failed".to_owned(),
                HealthStatus::Unhealthy,
            ),
        });
        self
    }

    pub fn add_redis_check<F, Fut>(mut self, probe: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        self.checks.push(HealthCheckDefinition {
            name: "redis".to_owned(),
            critical: false,
            check: boxed_check(
                probe,
                "Redis connection is healthy".to_owned(),
                "Redis connection unavailable".to_owned(),
                HealthStatus::Degraded,
            ),
        });
        self
    }

    pub fn add_external_service_check<F, Fut>(
        mut self,
        name: &str,
        critical: bool,
        probe: F,
    ) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        let failing = if critical {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        };
        self.checks.push(HealthCheckDefinition {
            name: name.to_owned(),
            critical,
            check: boxed_check(
                probe,
                format!("{name} is reachable"),
                format!("{name} is unreachable"),
                failing,
            ),
        });
        self
    }

    pub fn build(self) -> Vec<HealthCheckDefinition> {
        self.checks
    }
}
