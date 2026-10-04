use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEventType {
    #[serde(rename = "user.created")]
    UserCreated,
    #[serde(rename = "user.updated")]
    UserUpdated,
    #[serde(rename = "user.deleted")]
    UserDeleted,
    #[serde(rename = "order.created")]
    OrderCreated,
    #[serde(rename = "order.updated")]
    OrderUpdated,
    #[serde(rename = "order.completed")]
    OrderCompleted,
    #[serde(rename = "payment.success")]
    PaymentSuccess,
    #[serde(rename = "payment.failed")]
    PaymentFailed,
}

impl WebhookEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserCreated => "user.created",
            Self::UserUpdated => "user.updated",
            Self::UserDeleted => "user.deleted",
            Self::OrderCreated => "order.created",
            Self::OrderUpdated => "order.updated",
            Self::OrderCompleted => "order.completed",
            Self::PaymentSuccess => "payment.success",
            Self::PaymentFailed => "payment.failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookDeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Retrying,
}

impl WebhookDeliveryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Delivered => "delivered",
            Self::Failed => "failed",
            Self::Retrying => "retrying",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: WebhookEventType,
    pub timestamp: String,
    pub data: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookSubscriptionDto {
    pub url: String,
    pub secret: String,
    pub events: Vec<WebhookEventType>,
    pub retry_count: Option<i32>,
    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookSubscriptionDto {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<WebhookEventType>>,
    pub is_active: Option<bool>,
    pub retry_count: Option<i32>,
    pub timeout_seconds: Option<i32>,
}
