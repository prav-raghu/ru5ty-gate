use ru5ty_gate_auth::TokenService;

use crate::services::{
    AuthService, UserService, WebhookDeliveryService, WebhookSubscriptionService,
};

pub struct Services {
    pub token: TokenService,
    pub user: UserService,
    pub auth: AuthService,
    pub webhook_subscription: WebhookSubscriptionService,
    pub webhook_delivery: WebhookDeliveryService,
}
