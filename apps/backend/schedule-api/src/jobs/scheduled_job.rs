use std::time::Duration;

use async_trait::async_trait;

#[async_trait]
pub trait ScheduledJob: Send + Sync {
    fn name(&self) -> &'static str;

    fn interval(&self) -> Duration;

    async fn run(&self) -> Result<(), String>;
}
