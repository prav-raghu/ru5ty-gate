use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ValidateSessionResponseBody {
    pub allow: bool,
    pub session_seconds: Option<u64>,
    pub redirect_url: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyResponseBody {
    pub session_duration_secs: u64,
    pub redirect_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResponseBody {
    pub accepted: usize,
}
