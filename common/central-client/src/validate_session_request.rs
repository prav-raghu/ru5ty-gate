use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ValidateSessionRequest {
    pub mac: String,
    pub token: String,
    pub gateway_name: String,
    pub client_ip: String,
}
