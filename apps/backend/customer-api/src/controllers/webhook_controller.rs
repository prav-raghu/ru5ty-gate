use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ru5ty_gate_http::{ApiPath, ApiQuery, AppError, AuthUser, ValidatedJson};
use ru5ty_gate_types::ApiResponse;
use serde::Serialize;

use crate::schemas::{
    CreateWebhookSubscriptionRequest, DeliveriesQuery, ListSubscriptionsQuery,
    RetryWebhookDeliveryRequest, UpdateWebhookSubscriptionRequest, WebhookPath,
};
use crate::services::RetryOutcome;
use crate::types::AppState;

const DEFAULT_DELIVERY_LIMIT: i64 = 50;

fn found_or_404<T: Serialize>(body: ApiResponse<T>) -> Response {
    let status = if body.is_successful {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    (status, Json(body)).into_response()
}

pub struct WebhookController;

impl WebhookController {
    pub async fn create_subscription(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<CreateWebhookSubscriptionRequest>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .create_subscription(&body, user.id)
            .await?;
        Ok((StatusCode::CREATED, Json(result)).into_response())
    }

    pub async fn get_subscription(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<WebhookPath>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .get_subscription(path.id, user.id)
            .await?;
        Ok(found_or_404(result))
    }

    pub async fn list_subscriptions(
        State(state): State<AppState>,
        user: AuthUser,
        ApiQuery(query): ApiQuery<ListSubscriptionsQuery>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .list_subscriptions(user.id, query.active)
            .await?;
        Ok((StatusCode::OK, Json(result)).into_response())
    }

    pub async fn update_subscription(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<WebhookPath>,
        ValidatedJson(body): ValidatedJson<UpdateWebhookSubscriptionRequest>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .update_subscription(path.id, user.id, &body)
            .await?;
        Ok(found_or_404(result))
    }

    pub async fn delete_subscription(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<WebhookPath>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .delete_subscription(path.id, user.id)
            .await?;
        Ok(found_or_404(result))
    }

    pub async fn get_deliveries(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<WebhookPath>,
        ApiQuery(query): ApiQuery<DeliveriesQuery>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .get_deliveries(
                path.id,
                user.id,
                query.limit.unwrap_or(DEFAULT_DELIVERY_LIMIT),
            )
            .await?;
        Ok(found_or_404(result))
    }

    pub async fn regenerate_secret(
        State(state): State<AppState>,
        user: AuthUser,
        ApiPath(path): ApiPath<WebhookPath>,
    ) -> Result<Response, AppError> {
        let result = state
            .services
            .webhook_subscription
            .regenerate_secret(path.id, user.id)
            .await?;
        Ok(found_or_404(result))
    }

    pub async fn retry_delivery(
        State(state): State<AppState>,
        user: AuthUser,
        ValidatedJson(body): ValidatedJson<RetryWebhookDeliveryRequest>,
    ) -> Result<Response, AppError> {
        let outcome = state
            .services
            .webhook_delivery
            .retry_failed_delivery(body.delivery_id, user.id)
            .await?;
        match outcome {
            RetryOutcome::Started => Ok((
                StatusCode::OK,
                Json(ApiResponse::<()>::success_with_message(
                    (),
                    "Delivery retry initiated",
                )),
            )
                .into_response()),
            RetryOutcome::NotFound => Err(AppError::NotFound("Delivery not found".to_owned())),
            RetryOutcome::AlreadyDelivered => Err(AppError::BadRequest(
                "Cannot retry a successful delivery".to_owned(),
            )),
        }
    }
}
