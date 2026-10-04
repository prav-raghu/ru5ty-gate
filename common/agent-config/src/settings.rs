use std::path::Path;

use serde::Deserialize;

use crate::{
    CentralSettings, ConfigError, EnforcementSettings, FasSettings, HeartbeatSettings,
    LimitsSettings, PrivacySettings, ServerSettings, SessionSettings, SyncSettings, VenueSettings,
};

pub const CONFIG_PATH_ENV: &str = "RU5TY_GATE_CONFIG";

pub const DEFAULT_CONFIG_PATH: &str = "config/agent.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub venue: VenueSettings,
    pub server: ServerSettings,
    pub fas: FasSettings,
    #[serde(default)]
    pub central: CentralSettings,
    #[serde(default)]
    pub session: SessionSettings,
    #[serde(default)]
    pub heartbeat: HeartbeatSettings,
    #[serde(default)]
    pub sync: SyncSettings,
    #[serde(default)]
    pub privacy: PrivacySettings,
    #[serde(default)]
    pub limits: LimitsSettings,
    #[serde(default)]
    pub enforcement: EnforcementSettings,
}

impl Settings {
    pub fn parse(raw: &str, path: &str) -> Result<Self, ConfigError> {
        let settings: Self = toml::from_str(raw).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let display = path.display().to_string();
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: display.clone(),
            source,
        })?;
        Self::parse(&raw, &display)
    }

    pub fn load() -> Result<Self, ConfigError> {
        let path =
            std::env::var(CONFIG_PATH_ENV).unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_owned());
        Self::load_from(path)
    }
}
