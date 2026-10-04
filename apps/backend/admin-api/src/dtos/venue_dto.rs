use chrono::{DateTime, Utc};
use ru5ty_gate_database::Venue;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub session_duration_secs: i32,
    pub redirect_url: Option<String>,
    pub allow_new_sessions: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Venue> for VenueDto {
    fn from(venue: Venue) -> Self {
        Self {
            id: venue.id,
            code: venue.code,
            name: venue.name,
            session_duration_secs: venue.session_duration_secs,
            redirect_url: venue.redirect_url,
            allow_new_sessions: venue.allow_new_sessions,
            created_at: venue.created_at,
            updated_at: venue.updated_at,
        }
    }
}
