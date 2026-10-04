use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyResponse {
    pub session_duration_secs: u64,
    #[serde(default)]
    pub redirect_url: Option<String>,
}
