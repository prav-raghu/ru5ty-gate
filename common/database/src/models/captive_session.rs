use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::{Entity, SoftDeletable};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CaptiveSession {
    pub id: Uuid,
    pub venue_id: Uuid,
    pub gateway_id: Uuid,
    pub mac_identifier: String,
    pub client_identifier: Option<String>,
    pub gateway_name: Option<String>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub end_reason: Option<String>,
    pub is_active: bool,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for CaptiveSession {
    const TABLE: &'static str = "captive_sessions";
    const ORDER_BY: &'static str = "granted_at DESC, id";
}

impl SoftDeletable for CaptiveSession {}
