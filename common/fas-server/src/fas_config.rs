use secrecy::SecretString;

#[derive(Debug, Clone)]
pub struct FasConfig {
    pub venue_id: String,
    pub gateway_name: Option<String>,
    pub default_session_secs: u64,
    pub allow_offline: bool,
    pub faskey: SecretString,
    pub verify_client_ip: bool,
}
