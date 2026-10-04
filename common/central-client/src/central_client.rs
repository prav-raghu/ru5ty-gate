use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;

use crate::{
    CentralError, ClientConfig, HeartbeatRequest, PolicyResponse, Result, SyncBatchRequest,
    SyncBatchResponse, SyncEventDto, ValidateSessionRequest, ValidateSessionResponse,
};

#[derive(Debug, Clone)]
pub struct CentralClient {
    http: reqwest::Client,
    config: ClientConfig,
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
            Some(key) => builder.bearer_auth(key.expose_secret()),
            None => builder,
        }
    }

    async fn dispatch(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let response = self
            .authed(builder)
            .send()
            .await
            .map_err(CentralError::Request)?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let body = response.text().await.unwrap_or_default();
        Err(CentralError::Api {
            status: status.as_u16(),
            body,
        })
    }

    async fn send<T: DeserializeOwned>(&self, builder: reqwest::RequestBuilder) -> Result<T> {
        self.dispatch(builder)
            .await?
            .json::<T>()
            .await
            .map_err(CentralError::Request)
    }

    pub async fn validate_session(
        &self,
        request: &ValidateSessionRequest,
    ) -> Result<ValidateSessionResponse> {
        let builder = self.http.post(self.url("/sessions/validate")).json(request);
        self.send(builder).await
    }

    pub async fn fetch_policy(&self) -> Result<PolicyResponse> {
        let builder = self.http.get(self.url("/policy"));
        self.send(builder).await
    }

    pub async fn post_heartbeat(&self, heartbeat: &HeartbeatRequest) -> Result<()> {
        let builder = self.http.post(self.url("/heartbeat")).json(heartbeat);
        self.dispatch(builder).await.map(drop)
    }

    pub async fn sync_events(&self, events: Vec<SyncEventDto>) -> Result<SyncBatchResponse> {
        let request = SyncBatchRequest {
            venue_id: self.config.venue_id.clone(),
            events,
        };
        let builder = self.http.post(self.url("/sync")).json(&request);
        self.send(builder).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn url_joins_venue_and_path_correctly() {
        let client = CentralClient::new(ClientConfig {
            base_url: "https://api.example.com/".to_owned(),
            api_key: None,
            timeout: Duration::from_secs(2),
            venue_id: "venue-1".to_owned(),
        })
        .unwrap();

        assert_eq!(
            client.url("/heartbeat"),
            "https://api.example.com/v1/venues/venue-1/heartbeat"
        );
    }
}
