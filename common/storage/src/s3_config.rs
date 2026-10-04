use ru5ty_gate_config::{ConfigError, EnvReader};
use ru5ty_gate_types::StorageProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Config {
    pub provider: StorageProvider,
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub public_base_url: Option<String>,
}

impl S3Config {
    pub fn from_env(env: &EnvReader, provider: StorageProvider) -> Result<Self, ConfigError> {
        let prefix = match provider {
            StorageProvider::R2 => "R2",
            _ => "S3",
        };
        let key = |suffix: &str| format!("{prefix}_{suffix}");
        let endpoint = match provider {
            StorageProvider::R2 => env.optional("R2_ENDPOINT").or_else(|| {
                env.optional("R2_ACCOUNT_ID")
                    .map(|account| format!("https://{account}.r2.cloudflarestorage.com"))
            }),
            _ => env.optional("S3_ENDPOINT"),
        };
        Ok(Self {
            provider,
            bucket: env.required(&key("BUCKET"))?,
            region: env
                .optional(&key("REGION"))
                .unwrap_or_else(|| "auto".to_owned()),
            endpoint,
            access_key_id: env.required(&key("ACCESS_KEY_ID"))?,
            secret_access_key: env.required(&key("SECRET_ACCESS_KEY"))?,
            public_base_url: env.optional(&key("PUBLIC_URL")),
        })
    }
}
