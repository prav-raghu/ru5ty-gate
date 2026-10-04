use chrono::{DateTime, Utc};
use ru5ty_gate_database::CaptiveSession;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptiveSessionDto {
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
}

impl From<CaptiveSession> for CaptiveSessionDto {
    fn from(session: CaptiveSession) -> Self {
        Self {
            id: session.id,
            venue_id: session.venue_id,
            gateway_id: session.gateway_id,
            mac_identifier: session.mac_identifier,
            client_identifier: session.client_identifier,
            gateway_name: session.gateway_name,
            granted_at: session.granted_at,
            expires_at: session.expires_at,
            ended_at: session.ended_at,
            end_reason: session.end_reason,
        }
    }
}
