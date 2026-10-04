use std::time::Duration;

use secrecy::SecretString;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub api_key: Option<SecretString>,
    pub timeout: Duration,
    pub venue_id: String,
}
