use ru5ty_gate_auth::{AuthConfig, AuthConfigError};
use ru5ty_gate_config::{ConfigError, EnvReader};
use ru5ty_gate_database::DatabaseConfig;
use ru5ty_gate_email::EmailConfig;
use ru5ty_gate_types::TokenScope;
use ru5ty_gate_utilities::CryptoUtil;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceConfigError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Auth(#[from] AuthConfigError),
    #[error("APP_ENV must be 'development' or 'production', received '{0}'")]
    InvalidEnvironment(String),
    #[error("TWO_FACTOR_ENCRYPTION_KEY must be 64 hexadecimal characters")]
    InvalidTwoFactorKey,
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub production: bool,
    pub cors_origin: String,
    pub trusted_proxy_hops: usize,
    pub admin_web_url: String,
    pub redis_url: String,
    pub redis_tls_reject_unauthorized: bool,
    pub password_reset_expiration_minutes: i64,
    pub two_factor_encryption_key: String,
    pub admin_bootstrap_enabled: bool,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub email: EmailConfig,
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
        let two_factor_encryption_key = env.required("TWO_FACTOR_ENCRYPTION_KEY")?;
        if two_factor_encryption_key.len() != 64
            || CryptoUtil::from_hex_key(&two_factor_encryption_key).is_err()
        {
            return Err(ServiceConfigError::InvalidTwoFactorKey);
        }
        Ok(Self {
            port: env.parse_required("PORT")?,
            production,
            cors_origin: env.required("CORS_ORIGIN")?,
            trusted_proxy_hops: env.parse_or("TRUSTED_PROXY_HOPS", 1_usize)?,
            admin_web_url: env.required("ADMIN_WEB_URL")?,
            redis_url: env.required("REDIS_URL")?,
            redis_tls_reject_unauthorized: env
                .optional("REDIS_TLS_REJECT_UNAUTHORIZED")
                .is_none_or(|value| value != "false"),
            password_reset_expiration_minutes: env
                .parse_or("PASSWORD_RESET_EXPIRATION_MINUTES", 30)?,
            two_factor_encryption_key,
            admin_bootstrap_enabled: env
                .optional("ADMIN_BOOTSTRAP_ENABLED")
                .is_none_or(|value| value != "false"),
            database: DatabaseConfig::from_env(env)?,
            auth: AuthConfig::from_env(env, TokenScope::Admin)?,
            email: EmailConfig::from_env(env)?,
        })
    }
}
