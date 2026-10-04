use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct UserDetailsRecord {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub gender: Option<String>,
    pub age: Option<i32>,
    pub accept_terms_and_conditions: bool,
    pub allow_email_communications: bool,
    pub ip_address: String,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
    pub two_factor_enabled: bool,
    pub user_status_id: Uuid,
    pub role_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub modified_by: String,
    pub status_name: String,
    pub status_is_active: bool,
    pub status_created_at: DateTime<Utc>,
    pub status_updated_at: DateTime<Utc>,
    pub status_created_by: String,
    pub status_modified_by: String,
    pub role_name: String,
    pub role_is_active: bool,
    pub role_created_at: DateTime<Utc>,
    pub role_updated_at: DateTime<Utc>,
    pub role_created_by: String,
    pub role_modified_by: String,
}
