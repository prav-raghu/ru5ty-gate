use ru5ty_gate_auth::{AuthConfig, AuthConfigError};
use ru5ty_gate_config::{ConfigError, EnvReader};
use ru5ty_gate_database::DatabaseConfig;
use ru5ty_gate_email::EmailConfig;
use ru5ty_gate_types::TokenScope;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceConfigError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Auth(#[from] AuthConfigError),
    #[error("APP_ENV must be 'development' or 'production', received '{0}'")]
    InvalidEnvironment(String),
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub production: bool,
    pub cors_origin: String,
    pub customer_web_url: String,
    pub redis_url: String,
    pub redis_tls_reject_unauthorized: bool,
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
        Ok(Self {
            port: env.parse_required("PORT")?,
            production,
            cors_origin: env.required("CORS_ORIGIN")?,
            customer_web_url: env.required("CUSTOMER_WEB_URL")?,
            redis_url: env.required("REDIS_URL")?,
            redis_tls_reject_unauthorized: env
                .optional("REDIS_TLS_REJECT_UNAUTHORIZED")
                .is_some_and(|value| value == "true"),
            database: DatabaseConfig::from_env(env)?,
            auth: AuthConfig::from_env(env, TokenScope::Customer)?,
            email: EmailConfig::from_env(env)?,
        })
    }
}
