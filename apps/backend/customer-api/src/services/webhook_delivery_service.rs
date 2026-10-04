use ru5ty_gate_database::PgPool;
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::WebhookEventType;
use ru5ty_gate_webhooks::{TargetPolicy, WebhookDeliveryService as SharedDeliveryService};
use serde_json::{Map, Value};
use uuid::Uuid;

pub use ru5ty_gate_webhooks::{RetryOutcome, next_retry_delay_seconds};

#[derive(Clone)]
pub struct WebhookDeliveryService {
    shared: SharedDeliveryService,
}

impl WebhookDeliveryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            shared: SharedDeliveryService::new(pool),
        }
    }

    #[must_use]
    pub fn with_target_policy(mut self, target_policy: TargetPolicy) -> Self {
        self.shared = self.shared.with_target_policy(target_policy);
        self
    }

    pub async fn publish_event(
        &self,
        event_type: WebhookEventType,
        data: Map<String, Value>,
    ) -> Result<(), AppError> {
        self.shared
            .publish_event(event_type, data)
            .await
            .map_err(AppError::internal)
    }

    pub async fn process_deliveries(&self) -> Result<(), AppError> {
        self.shared
            .process_deliveries()
            .await
            .map_err(AppError::internal)
    }

    pub async fn retry_failed_delivery(
        &self,
        delivery_id: Uuid,
        user_id: Uuid,
    ) -> Result<RetryOutcome, AppError> {
        self.shared
            .retry_failed_delivery(delivery_id, user_id)
            .await
            .map_err(AppError::internal)
    }
}
