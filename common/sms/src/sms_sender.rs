use async_trait::async_trait;

#[async_trait]
pub trait SmsSender: Send + Sync {
    async fn send_sms(&self, to: &str, message: &str) -> bool;
}
