use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct PendingDelivery {
    pub id: Uuid,
    pub payload: Value,
    pub attempt_count: i32,
    pub url: String,
    pub secret: String,
    pub retry_count: i32,
    pub timeout_seconds: i32,
    pub status: String,
}
