use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub bind_addr: String,
    #[serde(default = "ServerSettings::default_admin_bind_addr")]
    pub admin_bind_addr: String,
    pub db_path: String,
}

impl ServerSettings {
    fn default_admin_bind_addr() -> String {
        "127.0.0.1:2081".to_owned()
    }
}
