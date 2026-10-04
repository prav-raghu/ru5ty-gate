use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmsPortalSendResponse {
    pub messages: Option<u32>,
    pub error_report: Option<SmsPortalErrorReport>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SmsPortalErrorReport {
    #[serde(default)]
    pub faults: Vec<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmsPortalResponse {
    #[serde(default)]
    pub errors: Vec<String>,
    pub send_response: Option<SmsPortalSendResponse>,
}

impl SmsPortalResponse {
    pub fn is_accepted(&self) -> bool {
        let Some(send) = &self.send_response else {
            return false;
        };
        let faults = send
            .error_report
            .as_ref()
            .map_or(0, |report| report.faults.len());
        self.errors.is_empty() && faults == 0 && send.messages.unwrap_or(0) >= 1
    }
}
