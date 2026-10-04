use std::time::Duration;

use ru5ty_gate_config::{ConfigError, EnvReader};
use ru5ty_gate_database::DatabaseConfig;
use thiserror::Error;

const MIN_API_KEY_LENGTH: usize = 32;

#[derive(Debug, Error)]
pub enum ServiceConfigError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("APP_ENV must be 'development' or 'production', received '{0}'")]
    InvalidEnvironment(String),
    #[error("SCHEDULE_API_KEY must be at least {MIN_API_KEY_LENGTH} characters")]
    WeakApiKey,
    #[error("SCHEDULE_WEBHOOK_INTERVAL_SECONDS must be greater than zero")]
    InvalidInterval,
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub production: bool,
    pub cors_origin: String,
    pub trusted_proxy_hops: usize,
    pub redis_url: String,
    pub redis_tls_reject_unauthorized: bool,
    pub schedule_api_key: String,
    pub webhook_interval: Duration,
    pub database: DatabaseConfig,
}

impl ServiceConfig {
    pub fn from_env(env: &EnvReader) -> Result<Self, ServiceConfigError> {
        let environment = env
            .optional("APP_ENV")
            .unwrap_or_else(|| "development".to_owned());
        let production = match environment.as_str() {
            "production" => true,
            "development" => false,
            other => return Err(ServiceConfigError::InvalidEnvironment(other.to_owned())),
        };
        let schedule_api_key = env.required("SCHEDULE_API_KEY")?;
        if schedule_api_key.len() < MIN_API_KEY_LENGTH {
            return Err(ServiceConfigError::WeakApiKey);
        }
        let interval_seconds: u64 = env.parse_or("SCHEDULE_WEBHOOK_INTERVAL_SECONDS", 60)?;
        if interval_seconds == 0 {
            return Err(ServiceConfigError::InvalidInterval);
        }
        Ok(Self {
            port: env.parse_required("PORT")?,
            production,
            cors_origin: env.required("CORS_ORIGIN")?,
            trusted_proxy_hops: env.parse_or("TRUSTED_PROXY_HOPS", 1_usize)?,
            redis_url: env.required("REDIS_URL")?,
            redis_tls_reject_unauthorized: env
                .optional("REDIS_TLS_REJECT_UNAUTHORIZED")
                .is_none_or(|value| value != "false"),
            schedule_api_key,
            webhook_interval: Duration::from_secs(interval_seconds),
            database: DatabaseConfig::from_env(env)?,
        })
    }
}
