use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PrivacySettings {
    #[serde(default)]
    pub send_raw_identifiers: bool,
    #[serde(default)]
    pub hash_pepper: Option<SecretString>,
}
