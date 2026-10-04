use sqlx::Postgres;
use sqlx::query_builder::Separated;

use crate::models::WebhookSubscription;
use crate::repositories::Writable;

#[derive(Debug, Clone)]
pub struct NewWebhookSubscription {
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub retry_count: i32,
    pub timeout_seconds: i32,
    pub created_by: String,
}

impl Writable for NewWebhookSubscription {
    type Entity = WebhookSubscription;
    const COLUMNS: &'static [&'static str] = &[
        "url",
        "secret",
        "events",
        "retry_count",
        "timeout_seconds",
        "created_by",
        "modified_by",
    ];

    fn bind_values(&self, values: &mut Separated<'_, Postgres, &'static str>) {
        values
            .push_bind(&self.url)
            .push_bind(&self.secret)
            .push_bind(&self.events)
            .push_bind(self.retry_count)
            .push_bind(self.timeout_seconds)
            .push_bind(&self.created_by)
            .push_bind(&self.created_by);
    }
}
