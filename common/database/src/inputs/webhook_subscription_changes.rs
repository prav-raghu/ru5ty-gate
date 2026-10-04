use sqlx::Postgres;
use sqlx::query_builder::Separated;

use crate::models::WebhookSubscription;
use crate::repositories::Writable;

#[derive(Debug, Clone)]
pub struct WebhookSubscriptionChanges {
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub retry_count: i32,
    pub timeout_seconds: i32,
    pub modified_by: String,
}

impl Writable for WebhookSubscriptionChanges {
    type Entity = WebhookSubscription;
    const COLUMNS: &'static [&'static str] = &[
        "url",
        "secret",
        "events",
        "is_active",
        "retry_count",
        "timeout_seconds",
        "modified_by",
    ];

    fn bind_values(&self, values: &mut Separated<'_, Postgres, &'static str>) {
        values
            .push_bind(&self.url)
            .push_bind(&self.secret)
            .push_bind(&self.events)
            .push_bind(self.is_active)
            .push_bind(self.retry_count)
            .push_bind(self.timeout_seconds)
            .push_bind(&self.modified_by);
    }
}
