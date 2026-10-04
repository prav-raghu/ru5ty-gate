use ru5ty_gate_config::{ConfigError, EnvReader};
use ru5ty_gate_types::TokenScope;
use thiserror::Error;

const MIN_SECRET_LENGTH: usize = 32;

#[derive(Debug, Error)]
pub enum AuthConfigError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("{key} must be at least {MIN_SECRET_LENGTH} characters")]
    WeakSecret { key: String },
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub access_secret: String,
    pub refresh_secret: String,
    pub scope: TokenScope,
}

impl AuthConfig {
    pub fn from_env(env: &EnvReader, scope: TokenScope) -> Result<Self, AuthConfigError> {
        let access_secret = env.required("JWT_SECRET")?;
        let refresh_secret = env.required("JWT_REFRESH_SECRET")?;
        for (key, value) in [
            ("JWT_SECRET", &access_secret),
            ("JWT_REFRESH_SECRET", &refresh_secret),
        ] {
            if value.len() < MIN_SECRET_LENGTH {
                return Err(AuthConfigError::WeakSecret {
                    key: key.to_owned(),
                });
            }
        }
        Ok(Self {
            access_secret,
            refresh_secret,
            scope,
        })
    }
}
