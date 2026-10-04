use ru5ty_gate_config::{ConfigError, EnvReader};
use thiserror::Error;

use crate::config::GraphqlConfig;

#[derive(Debug, Error)]
pub enum ServiceConfigError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("APP_ENV must be 'development' or 'production', received '{0}'")]
    InvalidEnvironment(String),
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    pub production: bool,
    pub cors_origin: String,
    pub customer_api_url: String,
    pub admin_api_url: String,
    pub scheduler_api_url: String,
    pub rate_limit_max: u32,
    pub graphql: GraphqlConfig,
}

fn url(env: &EnvReader, key: &str) -> Result<String, ConfigError> {
    Ok(env.required(key)?.trim_end_matches('/').to_owned())
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
            customer_api_url: url(env, "CUSTOMER_API_URL")?,
            admin_api_url: url(env, "ADMIN_API_URL")?,
            scheduler_api_url: url(env, "SCHEDULER_API_URL")?,
            rate_limit_max: env.parse_or("RATE_LIMIT_MAX", 200)?,
            graphql: GraphqlConfig::from_env(env, production),
        })
    }
}
