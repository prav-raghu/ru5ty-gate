use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::repositories::{Entity, SoftDeletable};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Gateway {
    pub id: Uuid,
    pub venue_id: Uuid,
    pub name: String,
    #[serde(skip_serializing)]
    pub api_key_hash: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub agent_version: Option<String>,
    pub uptime_secs: Option<i64>,
    pub active_sessions: Option<i32>,
    pub pending_events: Option<i32>,
    pub last_sync_ok_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_by: Option<String>,
    pub modified_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Entity for Gateway {
    const TABLE: &'static str = "gateways";
}

impl SoftDeletable for Gateway {}
