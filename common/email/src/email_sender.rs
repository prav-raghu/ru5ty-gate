use std::collections::BTreeMap;

use async_trait::async_trait;

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send_mail(
        &self,
        to: &str,
        subject: &str,
        template: &str,
        variables: &BTreeMap<String, String>,
    ) -> bool;
}
