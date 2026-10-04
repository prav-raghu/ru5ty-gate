use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NewCaptiveSession {
    pub venue_id: Uuid,
    pub gateway_id: Uuid,
    pub mac_identifier: String,
    pub client_identifier: Option<String>,
    pub gateway_name: Option<String>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
