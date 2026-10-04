use std::sync::Arc;

use ru5ty_gate_database::PgPool;
use ru5ty_gate_webhooks::{TargetPolicy, WebhookDeliveryService};

use crate::config::ServiceConfig;
use crate::jobs::{ScheduledJob, WebhookProcessorJob};
use crate::services::CronSchedulerService;

pub fn build_scheduler(config: &ServiceConfig, pool: &PgPool) -> CronSchedulerService {
    let webhook_processor: Arc<dyn ScheduledJob> = Arc::new(WebhookProcessorJob::new(
        WebhookDeliveryService::new(pool.clone())
            .with_target_policy(TargetPolicy::from_production(config.production)),
        config.webhook_interval,
    ));
    CronSchedulerService::new(vec![webhook_processor])
}
