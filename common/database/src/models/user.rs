use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::{Entity, SoftDeletable};

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub email: String,
    pub avatar: Option<String>,
    pub gender: Option<String>,
    pub age: Option<i32>,
    pub accept_terms_and_conditions: bool,
    pub allow_email_communications: bool,
    pub ip_address: String,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
    pub auth_hash: Option<String>,
    pub auth_hash_expiration: Option<DateTime<Utc>>,
    pub two_factor_enabled: bool,
    pub two_factor_secret: Option<String>,
    pub user_status_id: Uuid,
    pub role_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub modified_by: String,
}

impl Entity for User {
    const TABLE: &'static str = "users";
}

impl SoftDeletable for User {}
