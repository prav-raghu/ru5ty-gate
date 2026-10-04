use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub venue_id: String,
}
