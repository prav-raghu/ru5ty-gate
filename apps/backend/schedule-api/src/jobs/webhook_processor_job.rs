use std::time::Duration;

use async_trait::async_trait;
use ru5ty_gate_webhooks::WebhookDeliveryService;

use crate::jobs::ScheduledJob;

pub struct WebhookProcessorJob {
    deliveries: WebhookDeliveryService,
    interval: Duration,
}

impl WebhookProcessorJob {
    pub const NAME: &'static str = "webhookProcessor";

    pub fn new(deliveries: WebhookDeliveryService, interval: Duration) -> Self {
        Self {
            deliveries,
            interval,
        }
    }
}

#[async_trait]
impl ScheduledJob for WebhookProcessorJob {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn run(&self) -> Result<(), String> {
        self.deliveries
            .process_deliveries()
            .await
            .map_err(|error| error.to_string())
    }
}
