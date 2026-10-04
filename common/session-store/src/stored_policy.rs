#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPolicy {
    pub session_duration_secs: u64,
    pub redirect_url: Option<String>,
}
