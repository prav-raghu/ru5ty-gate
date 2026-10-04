use serde::Deserialize;
use uuid::Uuid;
use validator::{Validate, ValidationError};

fn validate_code(value: &str) -> Result<(), ValidationError> {
    let valid = (3..=64).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(ValidationError::new("venue_code"))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueRefPath {
    pub venue_ref: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayIdPath {
    pub gateway_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateVenueRequest {
    #[validate(custom(function = "validate_code"))]
    pub code: String,
    #[validate(length(min = 1, max = 120))]
    pub name: String,
    #[validate(range(min = 60, max = 604_800))]
    pub session_duration_secs: Option<i32>,
    #[validate(url, length(max = 2048))]
    pub redirect_url: Option<String>,
    pub allow_new_sessions: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateVenueRequest {
    #[validate(length(min = 1, max = 120))]
    pub name: Option<String>,
    #[validate(range(min = 60, max = 604_800))]
    pub session_duration_secs: Option<i32>,
    #[validate(url, length(max = 2048))]
    pub redirect_url: Option<String>,
    pub clear_redirect_url: Option<bool>,
    pub allow_new_sessions: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PageQuery {
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<i64>,
    #[validate(range(min = 0))]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SessionListQuery {
    pub open_only: Option<bool>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<i64>,
    #[validate(range(min = 0))]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CreateGatewayRequest {
    #[validate(length(min = 1, max = 80))]
    pub name: String,
}
