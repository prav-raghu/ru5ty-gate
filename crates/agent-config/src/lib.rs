//! Settings for the ru5ty-gate captive portal agent.
//!
//! Config is a single TOML file (see `config/agent.example.toml` in the repo
//! root). v1 is single-venue/single-gateway, so this is intentionally flat --
//! no per-venue arrays, no advertiser walled-garden config. That's deferred
//! to the full Gosling spec if this proves out.

use std::path::Path;

use serde::Deserialize;

/// Env var that overrides the default config file lookup path.
pub const CONFIG_PATH_ENV: &str = "RU5TY_GATE_CONFIG";

/// Default config path when nothing else is specified.
pub const DEFAULT_CONFIG_PATH: &str = "config/agent.toml";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub venue: VenueSettings,
    pub server: ServerSettings,
    #[serde(default)]
    pub central: CentralSettings,
    #[serde(default)]
    pub session: SessionSettings,
    #[serde(default)]
    pub heartbeat: HeartbeatSettings,
    #[serde(default)]
    pub sync: SyncSettings,
}

/// Identity of the single venue/gateway this agent instance serves.
#[derive(Debug, Clone, Deserialize)]
pub struct VenueSettings {
    /// Central-platform venue identifier.
    pub id: String,
    /// Human-readable name, mostly for logs.
    #[serde(default = "VenueSettings::default_name")]
    pub name: String,
    /// The openNDS `gatewayname` we expect on inbound FAS requests. If set,
    /// requests for a different gateway name are rejected -- v1 is
    /// single-gateway on purpose.
    #[serde(default)]
    pub gateway_name: Option<String>,
}

impl VenueSettings {
    fn default_name() -> String {
        "default".to_string()
    }
}

/// Where the FAS HTTP server listens and where its local DB lives.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    /// Address:port the axum server binds to, e.g. "0.0.0.0:2080".
    pub bind_addr: String,
    /// Path to the sqlite session-store database file.
    pub db_path: String,
}

/// Central platform API connection settings.
#[derive(Debug, Clone, Deserialize)]
pub struct CentralSettings {
    /// Base URL of the central platform API, e.g. "https://api.example.com".
    #[serde(default = "CentralSettings::default_base_url")]
    pub base_url: String,
    /// Bearer token / API key sent with every request, if configured.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Timeout for outbound calls to the central API. Kept short so an
    /// unreachable central platform never stalls the FAS auth round-trip --
    /// the agent falls back to local policy instead.
    #[serde(default = "CentralSettings::default_timeout_secs")]
    pub timeout_secs: u64,
}

impl CentralSettings {
    fn default_base_url() -> String {
        "http://127.0.0.1:8080".to_string()
    }
    fn default_timeout_secs() -> u64 {
        3
    }
}

impl Default for CentralSettings {
    fn default() -> Self {
        Self {
            base_url: Self::default_base_url(),
            api_key: None,
            timeout_secs: Self::default_timeout_secs(),
        }
    }
}

/// Fallback session policy used when the central platform can't be reached
/// to authorize a client, or hasn't overridden the defaults.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionSettings {
    /// How long a locally-granted session lasts, in seconds.
    #[serde(default = "SessionSettings::default_duration_secs")]
    pub default_duration_secs: u64,
    /// Whether to allow clients when the central API is unreachable
    /// (offline-first "fail open"), vs. denying them ("fail closed").
    #[serde(default = "SessionSettings::default_allow_offline")]
    pub allow_offline: bool,
}

impl SessionSettings {
    fn default_duration_secs() -> u64 {
        3600
    }
    fn default_allow_offline() -> bool {
        true
    }
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            default_duration_secs: Self::default_duration_secs(),
            allow_offline: Self::default_allow_offline(),
        }
    }
}

/// Heartbeat scheduling.
#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatSettings {
    #[serde(default = "HeartbeatSettings::default_interval_secs")]
    pub interval_secs: u64,
}

impl HeartbeatSettings {
    fn default_interval_secs() -> u64 {
        60
    }
}

impl Default for HeartbeatSettings {
    fn default() -> Self {
        Self {
            interval_secs: Self::default_interval_secs(),
        }
    }
}

/// Buffered session-event sync scheduling.
#[derive(Debug, Clone, Deserialize)]
pub struct SyncSettings {
    #[serde(default = "SyncSettings::default_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "SyncSettings::default_batch_size")]
    pub batch_size: u32,
}

impl SyncSettings {
    fn default_interval_secs() -> u64 {
        30
    }
    fn default_batch_size() -> u32 {
        100
    }
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            interval_secs: Self::default_interval_secs(),
            batch_size: Self::default_batch_size(),
        }
    }
}

impl Settings {
    /// Load settings from a specific TOML file.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.display().to_string(),
            source,
        })
    }

    /// Load settings from `$RU5TY_GATE_CONFIG` if set, else
    /// [`DEFAULT_CONFIG_PATH`].
    pub fn load() -> Result<Self, ConfigError> {
        let path =
            std::env::var(CONFIG_PATH_ENV).unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
        Self::load_from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let toml_str = r#"
            [venue]
            id = "venue-1"

            [server]
            bind_addr = "0.0.0.0:2080"
            db_path = "/var/lib/ru5ty-gate/sessions.db"
        "#;
        let settings: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(settings.venue.id, "venue-1");
        assert_eq!(settings.venue.name, "default");
        assert_eq!(settings.session.default_duration_secs, 3600);
        assert!(settings.session.allow_offline);
        assert_eq!(settings.heartbeat.interval_secs, 60);
        assert_eq!(settings.sync.batch_size, 100);
    }

    #[test]
    fn parses_full_config() {
        let toml_str = r#"
            [venue]
            id = "venue-1"
            name = "Test Venue"
            gateway_name = "gl-mt6000"

            [server]
            bind_addr = "0.0.0.0:2080"
            db_path = "sessions.db"

            [central]
            base_url = "https://central.example.com"
            api_key = "secret"
            timeout_secs = 5

            [session]
            default_duration_secs = 7200
            allow_offline = false

            [heartbeat]
            interval_secs = 30

            [sync]
            interval_secs = 15
            batch_size = 50
        "#;
        let settings: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(settings.venue.gateway_name.as_deref(), Some("gl-mt6000"));
        assert_eq!(settings.central.base_url, "https://central.example.com");
        assert_eq!(settings.central.api_key.as_deref(), Some("secret"));
        assert!(!settings.session.allow_offline);
        assert_eq!(settings.sync.interval_secs, 15);
    }
}
