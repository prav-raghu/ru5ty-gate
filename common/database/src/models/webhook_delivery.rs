use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::Entity;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub event_type: String,
    pub payload: Value,
    pub status: String,
    pub http_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub attempt_count: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for WebhookDelivery {
    const TABLE: &'static str = "webhook_deliveries";
}
