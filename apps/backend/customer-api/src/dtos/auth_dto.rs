use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub access_token: String,
    pub refresh_token: String,
    pub user_name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshData {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisterData {
    pub email: String,
}
