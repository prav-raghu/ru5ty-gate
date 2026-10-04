use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::{Entity, SoftDeletable};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Venue {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub session_duration_secs: i32,
    pub redirect_url: Option<String>,
    pub allow_new_sessions: bool,
    pub is_active: bool,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for Venue {
    const TABLE: &'static str = "venues";
}

impl SoftDeletable for Venue {}
