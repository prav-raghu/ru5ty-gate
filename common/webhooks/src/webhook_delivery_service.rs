use std::time::Duration;

use chrono::{DateTime, Utc};
use futures_util::future::join_all;
use reqwest::Client;
use ru5ty_gate_database::{PgPool, WebhookSubscription};
use ru5ty_gate_types::{WebhookDeliveryStatus, WebhookEventType, WebhookPayload};
use ru5ty_gate_utilities::WebhookSignatureService;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::pending_delivery::PendingDelivery;
use crate::target_policy::TargetPolicy;
use crate::webhook_error::WebhookError;

const BATCH_SIZE: i64 = 50;
const BASE_RETRY_DELAY_SECONDS: i64 = 60;
const MAX_RETRY_DELAY_SECONDS: i64 = 3600;
const RESPONSE_BODY_LIMIT: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOutcome {
    Started,
    NotFound,
    AlreadyDelivered,
}

struct AttemptResult {
    success: bool,
    http_status: Option<i32>,
    response_body: Option<String>,
    error_message: Option<String>,
}

#[derive(Clone)]
pub struct WebhookDeliveryService {
    pool: PgPool,
    client: Client,
    signatures: WebhookSignatureService,
    target_policy: TargetPolicy,
}

pub fn next_retry_delay_seconds(attempt_count: i32) -> i64 {
    let exponent = u32::try_from(attempt_count.saturating_sub(1))
        .unwrap_or(0)
        .min(20);
    BASE_RETRY_DELAY_SECONDS
        .saturating_mul(1_i64 << exponent)
        .min(MAX_RETRY_DELAY_SECONDS)
}

impl WebhookDeliveryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
            signatures: WebhookSignatureService,
            target_policy: TargetPolicy::AllowAll,
        }
    }

    #[must_use]
    pub fn with_target_policy(mut self, target_policy: TargetPolicy) -> Self {
        self.target_policy = target_policy;
        self
    }

    pub async fn publish_event(
        &self,
        event_type: WebhookEventType,
        data: Map<String, Value>,
    ) -> Result<(), WebhookError> {
        let payload = serde_json::to_value(WebhookPayload {
            event: event_type,
            timestamp: Utc::now().to_rfc3339(),
            data,
        })?;
        let subscriptions = sqlx::query_as::<_, WebhookSubscription>(
            "SELECT * FROM webhook_subscriptions WHERE is_active = TRUE AND $1 = ANY(events)",
        )
        .bind(event_type.as_str())
        .fetch_all(&self.pool)
        .await?;
        for subscription in subscriptions {
            sqlx::query(
                "INSERT INTO webhook_deliveries (subscription_id, event_type, payload, status, \
                 attempt_count) VALUES ($1, $2, $3, $4, 0)",
            )
            .bind(subscription.id)
            .bind(event_type.as_str())
            .bind(&payload)
            .bind(WebhookDeliveryStatus::Pending.as_str())
            .execute(&self.pool)
            .await?;
            sqlx::query("UPDATE webhook_subscriptions SET last_triggered_at = NOW() WHERE id = $1")
                .bind(subscription.id)
                .execute(&self.pool)
                .await?;
        }
        self.process_deliveries().await
    }

    pub async fn process_deliveries(&self) -> Result<(), WebhookError> {
        let pending = sqlx::query_as::<_, PendingDelivery>(
            "SELECT d.id, d.payload, d.attempt_count, s.url, s.secret, s.retry_count, \
             s.timeout_seconds, d.status \
             FROM webhook_deliveries d JOIN webhook_subscriptions s ON s.id = d.subscription_id \
             WHERE d.status IN ('pending', 'retrying') \
               AND (d.next_retry_at IS NULL OR d.next_retry_at <= NOW()) \
             ORDER BY d.created_at LIMIT $1",
        )
        .bind(BATCH_SIZE)
        .fetch_all(&self.pool)
        .await?;
        let total = pending.len();
        let results = join_all(pending.iter().map(|delivery| self.deliver(delivery))).await;
        let failed = results.iter().filter(|result| result.is_err()).count();
        if failed > 0 {
            tracing::warn!("Webhook delivery: {failed}/{total} deliveries failed");
        }
        Ok(())
    }

    pub async fn retry_failed_delivery(
        &self,
        delivery_id: Uuid,
        user_id: Uuid,
    ) -> Result<RetryOutcome, WebhookError> {
        let found = sqlx::query_as::<_, PendingDelivery>(
            "SELECT d.id, d.payload, d.attempt_count, s.url, s.secret, s.retry_count, \
             s.timeout_seconds, d.status \
             FROM webhook_deliveries d JOIN webhook_subscriptions s ON s.id = d.subscription_id \
             WHERE d.id = $1 AND s.created_by = $2",
        )
        .bind(delivery_id)
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = found else {
            return Ok(RetryOutcome::NotFound);
        };
        if row.status == WebhookDeliveryStatus::Delivered.as_str() {
            return Ok(RetryOutcome::AlreadyDelivered);
        }
        self.deliver(&row).await?;
        Ok(RetryOutcome::Started)
    }

    async fn deliver(&self, delivery: &PendingDelivery) -> Result<(), WebhookError> {
        let payload = serde_json::to_string(&delivery.payload)?;
        let signature = self
            .signatures
            .generate_signature(&payload, &delivery.secret);
        tracing::info!("Delivering webhook {} to {}", delivery.id, delivery.url);
        let result = self
            .attempt(&delivery.url, payload, &signature, delivery.timeout_seconds)
            .await;
        let attempt_count = delivery.attempt_count + 1;
        if result.success {
            self.mark_delivered(delivery.id, &result).await
        } else if attempt_count < delivery.retry_count {
            let next_retry_at =
                Utc::now() + chrono::Duration::seconds(next_retry_delay_seconds(attempt_count));
            self.schedule_retry(delivery.id, attempt_count, &result, next_retry_at)
                .await
        } else {
            self.mark_failed(delivery.id, attempt_count, &result).await
        }
    }

    async fn attempt(
        &self,
        url: &str,
        payload: String,
        signature: &str,
        timeout_seconds: i32,
    ) -> AttemptResult {
        if let Err(error) = self.target_policy.ensure_allowed(url).await {
            return AttemptResult {
                success: false,
                http_status: None,
                response_body: None,
                error_message: Some(error.to_string()),
            };
        }
        let timeout = Duration::from_secs(u64::try_from(timeout_seconds).unwrap_or(30));
        let response = self
            .client
            .post(url)
            .timeout(timeout)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .header("User-Agent", "WebhookService/1.0")
            .body(payload)
            .send()
            .await;
        match response {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                AttemptResult {
                    success: status.is_success(),
                    http_status: Some(i32::from(status.as_u16())),
                    response_body: Some(body.chars().take(RESPONSE_BODY_LIMIT).collect()),
                    error_message: None,
                }
            }
            Err(error) => {
                tracing::error!("Webhook delivery failed: {error}");
                AttemptResult {
                    success: false,
                    http_status: None,
                    response_body: None,
                    error_message: Some(error.to_string()),
                }
            }
        }
    }

    async fn mark_delivered(&self, id: Uuid, result: &AttemptResult) -> Result<(), WebhookError> {
        sqlx::query(
            "UPDATE webhook_deliveries SET status = $2, http_status = $3, response_body = $4, \
             delivered_at = NOW(), next_retry_at = NULL, attempt_count = attempt_count + 1 \
             WHERE id = $1",
        )
        .bind(id)
        .bind(WebhookDeliveryStatus::Delivered.as_str())
        .bind(result.http_status)
        .bind(result.response_body.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn schedule_retry(
        &self,
        id: Uuid,
        attempt_count: i32,
        result: &AttemptResult,
        next_retry_at: DateTime<Utc>,
    ) -> Result<(), WebhookError> {
        sqlx::query(
            "UPDATE webhook_deliveries SET status = $2, attempt_count = $3, http_status = $4, \
             error_message = $5, next_retry_at = $6 WHERE id = $1",
        )
        .bind(id)
        .bind(WebhookDeliveryStatus::Retrying.as_str())
        .bind(attempt_count)
        .bind(result.http_status)
        .bind(result.error_message.as_deref())
        .bind(next_retry_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_failed(
        &self,
        id: Uuid,
        attempt_count: i32,
        result: &AttemptResult,
    ) -> Result<(), WebhookError> {
        sqlx::query(
            "UPDATE webhook_deliveries SET status = $2, attempt_count = $3, http_status = $4, \
             error_message = $5, next_retry_at = NULL WHERE id = $1",
        )
        .bind(id)
        .bind(WebhookDeliveryStatus::Failed.as_str())
        .bind(attempt_count)
        .bind(result.http_status)
        .bind(result.error_message.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
