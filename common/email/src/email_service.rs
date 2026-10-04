use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use reqwest::Client;
use serde_json::json;

use crate::email_config::EmailConfig;
use crate::email_sender::EmailSender;
use crate::template::render_template;

#[derive(Clone)]
pub struct EmailService {
    client: Client,
    config: EmailConfig,
}

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl EmailSender for EmailService {
    async fn send_mail(
        &self,
        to: &str,
        subject: &str,
        template: &str,
        variables: &BTreeMap<String, String>,
    ) -> bool {
        let Some(api_key) = self.config.api_key.as_deref() else {
            tracing::warn!("No email provider configured, set MAILTRAP_API_KEY");
            return false;
        };
        let Some(from_email) = self.config.from_email.as_deref() else {
            tracing::error!("MAILTRAP_FROM is not set");
            return false;
        };
        let html = match render_template(template, variables, Utc::now().year()) {
            Ok(html) => html,
            Err(error) => {
                tracing::error!(%error, "failed to render email template");
                return false;
            }
        };
        let body = json!({
            "from": { "email": from_email, "name": self.config.from_name },
            "to": [{ "email": to }],
            "subject": subject,
            "html": html,
        });
        match self
            .client
            .post(self.config.send_url())
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                tracing::info!(to, subject, "email sent via Mailtrap");
                true
            }
            Ok(response) => {
                tracing::error!(status = %response.status(), "Mailtrap rejected the email");
                false
            }
            Err(error) => {
                tracing::error!(%error, "failed to send email via Mailtrap");
                false
            }
        }
    }
}
