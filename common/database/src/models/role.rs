use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::{Entity, SoftDeletable};

#[derive(Debug, Clone, FromRow)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub modified_by: String,
}

impl Entity for Role {
    const TABLE: &'static str = "roles";
}

impl SoftDeletable for Role {}
