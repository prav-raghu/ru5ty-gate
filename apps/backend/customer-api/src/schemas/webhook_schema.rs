use ru5ty_gate_types::WebhookEventType;
use serde::Deserialize;
use uuid::Uuid;
use validator::{Validate, ValidationError};

fn known_events(events: &[String]) -> Result<(), ValidationError> {
    let known = [
        WebhookEventType::UserCreated,
        WebhookEventType::UserUpdated,
        WebhookEventType::UserDeleted,
        WebhookEventType::OrderCreated,
        WebhookEventType::OrderUpdated,
        WebhookEventType::OrderCompleted,
        WebhookEventType::PaymentSuccess,
        WebhookEventType::PaymentFailed,
    ];
    if events
        .iter()
        .all(|event| known.iter().any(|candidate| candidate.as_str() == event))
    {
        Ok(())
    } else {
        Err(ValidationError::new("events").with_message("Contains an unknown event type".into()))
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateWebhookSubscriptionRequest {
    #[validate(url)]
    pub url: String,
    #[validate(length(min = 32))]
    pub secret: Option<String>,
    #[validate(length(min = 1), custom(function = "known_events"))]
    pub events: Vec<String>,
    #[validate(range(min = 0, max = 10))]
    pub retry_count: Option<i32>,
    #[validate(range(min = 5, max = 300))]
    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateWebhookSubscriptionRequest {
    #[validate(url)]
    pub url: Option<String>,
    #[validate(length(min = 32))]
    pub secret: Option<String>,
    #[validate(length(min = 1), custom(function = "known_events"))]
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
    #[validate(range(min = 0, max = 10))]
    pub retry_count: Option<i32>,
    #[validate(range(min = 5, max = 300))]
    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPath {
    pub id: Uuid,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ListSubscriptionsQuery {
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct DeliveriesQuery {
    #[validate(range(min = 1, max = 500))]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RetryWebhookDeliveryRequest {
    pub delivery_id: Uuid,
}
