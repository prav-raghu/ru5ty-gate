use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub bind_addr: String,
    pub db_path: String,
}
