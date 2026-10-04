use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct UserSummaryRecord {
    pub id: Uuid,
    pub username: String,
    pub age: Option<i32>,
    pub last_seen: DateTime<Utc>,
}
