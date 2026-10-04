use std::time::Duration;

use ru5ty_gate_config::{ConfigError, EnvReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

impl DatabaseConfig {
    pub fn from_env(env: &EnvReader) -> Result<Self, ConfigError> {
        Ok(Self {
            url: env.required("DATABASE_URL")?,
            max_connections: env.parse_or("DATABASE_CONNECTION_LIMIT", 10)?,
            acquire_timeout: Duration::from_secs(env.parse_or("DATABASE_POOL_TIMEOUT", 10)?),
        })
    }
}
