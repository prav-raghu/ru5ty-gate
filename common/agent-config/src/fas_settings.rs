use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct FasSettings {
    pub faskey: SecretString,
    #[serde(default = "FasSettings::default_verify_client_ip")]
    pub verify_client_ip: bool,
}

impl FasSettings {
    fn default_verify_client_ip() -> bool {
        true
    }
}
