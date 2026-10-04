use axum::Router;
use axum::routing::{get, post};

use crate::controllers::WebhookController;
use crate::types::AppState;

pub struct WebhookRoutes;

impl WebhookRoutes {
    pub fn protected() -> Router<AppState> {
        Router::new()
            .route(
                "/webhooks/subscriptions",
                post(WebhookController::create_subscription)
                    .get(WebhookController::list_subscriptions),
            )
            .route(
                "/webhooks/subscriptions/{id}",
                get(WebhookController::get_subscription)
                    .put(WebhookController::update_subscription)
                    .delete(WebhookController::delete_subscription),
            )
            .route(
                "/webhooks/subscriptions/{id}/deliveries",
                get(WebhookController::get_deliveries),
            )
            .route(
                "/webhooks/subscriptions/{id}/regenerate-secret",
                post(WebhookController::regenerate_secret),
            )
            .route("/webhooks/retry", post(WebhookController::retry_delivery))
    }
}
