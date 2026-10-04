use std::net::SocketAddr;

use secrecy::ExposeSecret;

use crate::{ConfigError, Settings};

const MIN_SECRET_LENGTH: usize = 16;
const MAX_VENUE_ID_LENGTH: usize = 64;
const MAX_BATCH_SIZE: u32 = 1000;

fn invalid(field: &'static str, reason: &str) -> ConfigError {
    ConfigError::Invalid {
        field,
        reason: reason.to_owned(),
    }
}

fn require_at_least_one(field: &'static str, value: u64) -> Result<(), ConfigError> {
    if value == 0 {
        return Err(invalid(field, "must be at least 1"));
    }
    Ok(())
}

fn require_socket_addr(field: &'static str, value: &str) -> Result<SocketAddr, ConfigError> {
    value
        .parse::<SocketAddr>()
        .map_err(|_| invalid(field, "must be an ip:port socket address"))
}

fn require_secret(field: &'static str, value: &str) -> Result<(), ConfigError> {
    if value.trim().len() < MIN_SECRET_LENGTH {
        return Err(invalid(field, "must be at least 16 characters"));
    }
    Ok(())
}

fn url_host(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split('/').next()?;
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    if let Some(stripped) = host.strip_prefix('[') {
        return stripped.split(']').next();
    }
    host.split(':').next()
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

impl Settings {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_identity_and_addresses()?;
        self.validate_intervals_and_limits()?;
        self.validate_secrets_and_transport()
    }

    fn validate_identity_and_addresses(&self) -> Result<(), ConfigError> {
        let id = &self.venue.id;
        let valid_id = !id.is_empty()
            && id.len() <= MAX_VENUE_ID_LENGTH
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid_id {
            return Err(invalid(
                "venue.id",
                "must be 1 to 64 characters of letters, digits, '-' or '_'",
            ));
        }
        let public = require_socket_addr("server.bind_addr", &self.server.bind_addr)?;
        let admin = require_socket_addr("server.admin_bind_addr", &self.server.admin_bind_addr)?;
        if public == admin {
            return Err(invalid(
                "server.admin_bind_addr",
                "must differ from server.bind_addr",
            ));
        }
        if self.server.db_path.trim().is_empty() {
            return Err(invalid("server.db_path", "must not be empty"));
        }
        Ok(())
    }

    fn validate_intervals_and_limits(&self) -> Result<(), ConfigError> {
        require_at_least_one("central.timeout_secs", self.central.timeout_secs)?;
        require_at_least_one(
            "session.default_duration_secs",
            self.session.default_duration_secs,
        )?;
        require_at_least_one("heartbeat.interval_secs", self.heartbeat.interval_secs)?;
        require_at_least_one("sync.interval_secs", self.sync.interval_secs)?;
        require_at_least_one(
            "limits.request_timeout_secs",
            self.limits.request_timeout_secs,
        )?;
        require_at_least_one(
            "limits.max_concurrent_requests",
            self.limits.max_concurrent_requests as u64,
        )?;
        require_at_least_one(
            "limits.fas_requests_per_minute",
            u64::from(self.limits.fas_requests_per_minute),
        )?;
        if self.sync.batch_size == 0 || self.sync.batch_size > MAX_BATCH_SIZE {
            return Err(invalid("sync.batch_size", "must be between 1 and 1000"));
        }
        if self.sync.max_pending < self.sync.batch_size {
            return Err(invalid(
                "sync.max_pending",
                "must be at least sync.batch_size",
            ));
        }
        Ok(())
    }

    fn validate_secrets_and_transport(&self) -> Result<(), ConfigError> {
        require_secret("fas.faskey", self.fas.faskey.expose_secret())?;

        let base_url = &self.central.base_url;
        let host = url_host(base_url)
            .ok_or_else(|| invalid("central.base_url", "must start with http:// or https://"))?;
        if let Some(key) = &self.central.api_key {
            require_secret("central.api_key", key.expose_secret())?;
            let plain_http = base_url.starts_with("http://");
            if plain_http && !is_loopback_host(host) && !self.central.allow_insecure_http {
                return Err(invalid(
                    "central.base_url",
                    "must use https when central.api_key is set",
                ));
            }
        }

        if !self.privacy.send_raw_identifiers {
            let pepper = self.privacy.hash_pepper.as_ref().ok_or_else(|| {
                invalid(
                    "privacy.hash_pepper",
                    "required unless send_raw_identifiers is true",
                )
            })?;
            require_secret("privacy.hash_pepper", pepper.expose_secret())?;
        }
        Ok(())
    }
}
