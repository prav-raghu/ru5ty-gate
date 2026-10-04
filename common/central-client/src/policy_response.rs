use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyResponse {
    pub session_duration_secs: u64,
    #[serde(default)]
    pub redirect_url: Option<String>,
}
