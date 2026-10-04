use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateSessionResponse {
    pub allow: bool,
    #[serde(default)]
    pub session_seconds: Option<u64>,
    #[serde(default)]
    pub redirect_url: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}
