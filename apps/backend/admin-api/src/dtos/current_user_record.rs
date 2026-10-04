use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct CurrentUserRecord {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub allow_email_communications: bool,
    pub two_factor_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub role_id: Uuid,
    pub role_name: String,
    pub status_id: Uuid,
    pub status_name: String,
}
