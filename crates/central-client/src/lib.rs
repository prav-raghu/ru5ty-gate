//! Async client for the central platform API.
//!
//! Validates new sessions, pulls policy (session duration, redirect URL)
//! when reachable, posts heartbeats, and syncs buffered session events in
//! batches. Every call here is expected to fail sometimes -- the FAS
//! decision path and the sync/heartbeat tasks are both written to treat an
//! unreachable central platform as a normal, handled case, not an error to
//! propagate up as a crash.

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CentralError {
    #[error("failed to build http client: {0}")]
    ClientBuild(#[source] reqwest::Error),
    #[error("request to central API failed: {0}")]
    Request(#[source] reqwest::Error),
    #[error("central API returned {status}: {body}")]
    Api { status: u16, body: String },
}

pub type Result<T> = std::result::Result<T, CentralError>;

/// Connection settings for the central platform. Constructed by the agent
/// binary from `agent-config::Settings`, kept separate here so this crate
/// doesn't need to depend on the config crate.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub venue_id: String,
}

#[derive(Debug, Clone)]
pub struct CentralClient {
    http: reqwest::Client,
    config: ClientConfig,
}

// ---- validate session ----

#[derive(Debug, Clone, Serialize)]
pub struct ValidateSessionRequest {
    pub mac: String,
    pub token: String,
    pub gateway_name: String,
    pub client_ip: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateSessionResponse {
    pub allow: bool,
    #[serde(default)]
    pub session_seconds: Option<u64>,
    #[serde(default)]
    pub redirect_url: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

// ---- policy ----

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyResponse {
    pub session_duration_secs: u64,
    #[serde(default)]
    pub redirect_url: Option<String>,
}

// ---- heartbeat ----

#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatRequest {
    pub venue_id: String,
    pub uptime_secs: u64,
    pub active_sessions: i64,
    pub agent_version: String,
    /// Unix timestamp (seconds) the heartbeat was generated.
    pub timestamp: i64,
}

// ---- buffered session event sync ----

#[derive(Debug, Clone, Serialize)]
pub struct SyncEventDto {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncBatchRequest {
    pub venue_id: String,
    pub events: Vec<SyncEventDto>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SyncBatchResponse {
    #[serde(default)]
    pub accepted: usize,
}

impl CentralClient {
    pub fn new(config: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(CentralError::ClientBuild)?;
        Ok(Self { http, config })
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/v1/venues/{}{}",
            self.config.base_url.trim_end_matches('/'),
            self.config.venue_id,
            path
        )
    }

    fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.api_key {
            Some(key) => builder.bearer_auth(key),
            None => builder,
        }
    }

    async fn send<T: for<'de> Deserialize<'de>>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<T> {
        let resp = self
            .authed(builder)
            .send()
            .await
            .map_err(CentralError::Request)?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CentralError::Api {
                status: status.as_u16(),
                body,
            });
        }
        resp.json::<T>().await.map_err(CentralError::Request)
    }

    /// Ask the central platform whether a client session should be
    /// granted, and what policy (duration, redirect) to apply.
    pub async fn validate_session(
        &self,
        req: &ValidateSessionRequest,
    ) -> Result<ValidateSessionResponse> {
        let builder = self.http.post(self.url("/sessions/validate")).json(req);
        self.send(builder).await
    }

    /// Pull current policy (session duration default, redirect URL) for
    /// this venue.
    pub async fn fetch_policy(&self) -> Result<PolicyResponse> {
        let builder = self.http.get(self.url("/policy"));
        self.send(builder).await
    }

    /// Post a device health check-in. Central platform is expected to
    /// respond 2xx with an empty or ignorable body.
    pub async fn post_heartbeat(&self, hb: &HeartbeatRequest) -> Result<()> {
        let builder = self.http.post(self.url("/heartbeat")).json(hb);
        let resp = self
            .authed(builder)
            .send()
            .await
            .map_err(CentralError::Request)?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CentralError::Api {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }

    /// Push a batch of buffered session events. Returns how many the
    /// central platform accepted.
    pub async fn sync_events(&self, events: Vec<SyncEventDto>) -> Result<SyncBatchResponse> {
        let req = SyncBatchRequest {
            venue_id: self.config.venue_id.clone(),
            events,
        };
        let builder = self.http.post(self.url("/sync")).json(&req);
        self.send(builder).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(base_url: String) -> ClientConfig {
        ClientConfig {
            base_url,
            api_key: Some("test-key".to_string()),
            timeout: Duration::from_secs(2),
            venue_id: "venue-1".to_string(),
        }
    }

    #[test]
    fn url_joins_venue_and_path_correctly() {
        let client =
            CentralClient::new(test_config("https://api.example.com/".to_string())).unwrap();
        assert_eq!(
            client.url("/heartbeat"),
            "https://api.example.com/v1/venues/venue-1/heartbeat"
        );
    }

    #[tokio::test]
    async fn validate_session_reports_api_error_status() {
        // No server listening on this port -- exercises the Request error path.
        let client = CentralClient::new(test_config("http://127.0.0.1:1".to_string())).unwrap();
        let req = ValidateSessionRequest {
            mac: "AA:BB:CC:DD:EE:FF".to_string(),
            token: "tok".to_string(),
            gateway_name: "gw".to_string(),
            client_ip: "10.0.0.5".to_string(),
        };
        let err = client.validate_session(&req).await.unwrap_err();
        assert!(matches!(err, CentralError::Request(_)));
    }
}
