use std::sync::Arc;

use ru5ty_gate_auth::TokenService;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_database::PgPool;
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_webhooks::TargetPolicy;

use crate::config::ServiceConfig;
use crate::services::{
    AuthService, UserService, WebhookDeliveryService, WebhookSubscriptionService,
};
use crate::types::Services;

pub fn build_services(
    config: &ServiceConfig,
    pool: &PgPool,
    redis: &RedisService,
    email: Arc<dyn EmailSender>,
) -> Services {
    let target_policy = TargetPolicy::from_production(config.production);
    let token = TokenService::new(config.auth.clone(), redis.clone());
    let user = UserService::new(pool.clone());
    let auth = AuthService::new(
        pool.clone(),
        token.clone(),
        email,
        redis.clone(),
        config.customer_web_url.clone(),
    );
    Services {
        token,
        user,
        auth,
        webhook_subscription: WebhookSubscriptionService::new(pool.clone())
            .with_target_policy(target_policy),
        webhook_delivery: WebhookDeliveryService::new(pool.clone())
            .with_target_policy(target_policy),
    }
}
