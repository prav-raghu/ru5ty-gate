use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::{Entity, SoftDeletable};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WebhookSubscription {
    pub id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub retry_count: i32,
    pub timeout_seconds: i32,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_triggered_at: Option<DateTime<Utc>>,
}

impl Entity for WebhookSubscription {
    const TABLE: &'static str = "webhook_subscriptions";
}

impl SoftDeletable for WebhookSubscription {}
