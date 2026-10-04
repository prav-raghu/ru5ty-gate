use async_trait::async_trait;
use serde_json::Value;

use crate::job::Job;

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle(&self, job: &Job) -> Result<Value, String>;
}
