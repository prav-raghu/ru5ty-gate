use ru5ty_gate_database::{
    NewWebhookSubscription, PgPool, Repository, WebhookDelivery, WebhookSubscription, sqlx,
};
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::ApiResponse;
use ru5ty_gate_utilities::WebhookSignatureService;
use ru5ty_gate_webhooks::TargetPolicy;
use uuid::Uuid;

use crate::schemas::{CreateWebhookSubscriptionRequest, UpdateWebhookSubscriptionRequest};

const SUBSCRIPTION_NOT_FOUND: &str = "Subscription not found";
const TARGET_NOT_ALLOWED: &str = "Webhook URL must be a public http or https address";
const DEFAULT_RETRY_COUNT: i32 = 3;
const DEFAULT_TIMEOUT_SECONDS: i32 = 30;

#[derive(Clone)]
pub struct WebhookSubscriptionService {
    pool: PgPool,
    signatures: WebhookSignatureService,
    target_policy: TargetPolicy,
}

impl WebhookSubscriptionService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            signatures: WebhookSignatureService,
            target_policy: TargetPolicy::AllowAll,
        }
    }

    #[must_use]
    pub fn with_target_policy(mut self, target_policy: TargetPolicy) -> Self {
        self.target_policy = target_policy;
        self
    }

    pub async fn create_subscription(
        &self,
        request: &CreateWebhookSubscriptionRequest,
        created_by: Uuid,
    ) -> Result<ApiResponse<WebhookSubscription>, AppError> {
        if self
            .target_policy
            .ensure_allowed(&request.url)
            .await
            .is_err()
        {
            return Ok(ApiResponse::failure(TARGET_NOT_ALLOWED));
        }
        let secret = request
            .secret
            .clone()
            .unwrap_or_else(|| self.signatures.generate_secret());
        let subscription = Repository::<WebhookSubscription>::insert(
            &self.pool,
            &NewWebhookSubscription {
                url: request.url.clone(),
                secret,
                events: request.events.clone(),
                retry_count: request.retry_count.unwrap_or(DEFAULT_RETRY_COUNT),
                timeout_seconds: request.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS),
                created_by: created_by.to_string(),
            },
        )
        .await
        .map_err(AppError::internal)?;
        Ok(ApiResponse::success(subscription))
    }

    pub async fn get_subscription(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<ApiResponse<WebhookSubscription>, AppError> {
        let found = self.find_owned(id, user_id).await?;
        Ok(found.map_or_else(
            || ApiResponse::failure(SUBSCRIPTION_NOT_FOUND),
            ApiResponse::success,
        ))
    }

    async fn find_owned(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<WebhookSubscription>, AppError> {
        sqlx::query_as::<_, WebhookSubscription>(
            "SELECT * FROM webhook_subscriptions WHERE id = $1 AND created_by = $2",
        )
        .bind(id)
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)
    }

    pub async fn list_subscriptions(
        &self,
        user_id: Uuid,
        is_active: Option<bool>,
    ) -> Result<ApiResponse<Vec<WebhookSubscription>>, AppError> {
        let subscriptions = sqlx::query_as::<_, WebhookSubscription>(
            "SELECT * FROM webhook_subscriptions \
             WHERE created_by = $1 AND ($2::boolean IS NULL OR is_active = $2) \
             ORDER BY created_at DESC",
        )
        .bind(user_id.to_string())
        .bind(is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(ApiResponse::success(subscriptions))
    }

    pub async fn update_subscription(
        &self,
        id: Uuid,
        user_id: Uuid,
        request: &UpdateWebhookSubscriptionRequest,
    ) -> Result<ApiResponse<WebhookSubscription>, AppError> {
        if let Some(url) = request.url.as_deref()
            && self.target_policy.ensure_allowed(url).await.is_err()
        {
            return Ok(ApiResponse::failure(TARGET_NOT_ALLOWED));
        }
        let updated = sqlx::query_as::<_, WebhookSubscription>(
            "UPDATE webhook_subscriptions SET \
             url = COALESCE($3, url), \
             secret = COALESCE($4, secret), \
             events = COALESCE($5, events), \
             is_active = COALESCE($6, is_active), \
             retry_count = COALESCE($7, retry_count), \
             timeout_seconds = COALESCE($8, timeout_seconds), \
             modified_by = $2 \
             WHERE id = $1 AND created_by = $2 RETURNING *",
        )
        .bind(id)
        .bind(user_id.to_string())
        .bind(request.url.as_deref())
        .bind(request.secret.as_deref())
        .bind(request.events.as_deref())
        .bind(request.is_active)
        .bind(request.retry_count)
        .bind(request.timeout_seconds)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(updated.map_or_else(
            || ApiResponse::failure(SUBSCRIPTION_NOT_FOUND),
            ApiResponse::success,
        ))
    }

    pub async fn delete_subscription(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<ApiResponse<serde_json::Value>, AppError> {
        let deleted =
            sqlx::query("DELETE FROM webhook_subscriptions WHERE id = $1 AND created_by = $2")
                .bind(id)
                .bind(user_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(AppError::internal)?;
        if deleted.rows_affected() == 0 {
            return Ok(ApiResponse::failure(SUBSCRIPTION_NOT_FOUND));
        }
        Ok(ApiResponse::success(serde_json::json!({ "id": id })))
    }

    pub async fn get_deliveries(
        &self,
        subscription_id: Uuid,
        user_id: Uuid,
        limit: i64,
    ) -> Result<ApiResponse<Vec<WebhookDelivery>>, AppError> {
        if self.find_owned(subscription_id, user_id).await?.is_none() {
            let mut response = ApiResponse::failure(SUBSCRIPTION_NOT_FOUND);
            response.data = Some(Vec::new());
            return Ok(response);
        }
        let deliveries = sqlx::query_as::<_, WebhookDelivery>(
            "SELECT * FROM webhook_deliveries WHERE subscription_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(subscription_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(ApiResponse::success(deliveries))
    }

    pub async fn regenerate_secret(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<ApiResponse<WebhookSubscription>, AppError> {
        let updated = sqlx::query_as::<_, WebhookSubscription>(
            "UPDATE webhook_subscriptions SET secret = $3, modified_by = $2 \
             WHERE id = $1 AND created_by = $2 RETURNING *",
        )
        .bind(id)
        .bind(user_id.to_string())
        .bind(self.signatures.generate_secret())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(updated.map_or_else(
            || ApiResponse::failure(SUBSCRIPTION_NOT_FOUND),
            ApiResponse::success,
        ))
    }
}
