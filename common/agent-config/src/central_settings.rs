use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CentralSettings {
    #[serde(default = "CentralSettings::default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "CentralSettings::default_timeout_secs")]
    pub timeout_secs: u64,
}

impl CentralSettings {
    fn default_base_url() -> String {
        "http://127.0.0.1:8080".to_owned()
    }

    fn default_timeout_secs() -> u64 {
        3
    }
}

impl Default for CentralSettings {
    fn default() -> Self {
        Self {
            base_url: Self::default_base_url(),
            api_key: None,
            timeout_secs: Self::default_timeout_secs(),
        }
    }
}
