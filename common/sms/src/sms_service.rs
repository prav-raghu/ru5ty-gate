use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::Client;
use serde_json::{Value, json};

use crate::phone::to_e164;
use crate::sms_config::SmsConfig;
use crate::sms_portal_response::SmsPortalResponse;
use crate::sms_sender::SmsSender;

const SMSPORTAL_SEND_URL: &str = "https://rest.smsportal.com/v3/BulkMessages";

#[derive(Clone)]
pub struct SmsService {
    client: Client,
    config: SmsConfig,
}

impl SmsService {
    pub fn new(config: SmsConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl SmsSender for SmsService {
    async fn send_sms(&self, to: &str, message: &str) -> bool {
        if !self.config.enabled {
            tracing::info!(to, "SMS disabled - skipping");
            return false;
        }
        let (Some(client_id), Some(api_secret)) = (
            self.config.client_id.as_deref(),
            self.config.api_secret.as_deref(),
        ) else {
            tracing::error!("SMSPORTAL_CLIENT_ID or SMSPORTAL_API_SECRET is not set");
            return false;
        };
        let Some(destination) = to_e164(to) else {
            tracing::error!(to, "Invalid phone number - cannot convert to E164");
            return false;
        };

        let mut body: Value = json!({
            "messages": [{ "content": message, "destination": destination }]
        });
        if let Some(sender_id) = &self.config.sender_id {
            body["sendOptions"] = json!({ "senderId": sender_id });
        }
        let credentials = STANDARD.encode(format!("{client_id}:{api_secret}"));

        let response = match self
            .client
            .post(SMSPORTAL_SEND_URL)
            .header("Authorization", format!("Basic {credentials}"))
            .json(&body)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                tracing::error!(%error, "Unexpected error sending SMS via SMSPortal");
                return false;
            }
        };

        let status = response.status();
        let parsed = response
            .json::<SmsPortalResponse>()
            .await
            .unwrap_or_default();
        if !status.is_success() || !parsed.is_accepted() {
            tracing::error!(%status, to, "SMSPortal rejected the request");
            return false;
        }
        tracing::info!(to = destination, "SMS sent successfully");
        true
    }
}
