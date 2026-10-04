use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 30))]
    pub username: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(range(min = 13, max = 120))]
    pub age: i32,
    pub gender_id: String,
    pub accept_terms_and_conditions: bool,
    #[serde(default)]
    pub allow_email_communications: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(length(min = 3, max = 30))]
    pub username: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[serde(default)]
    pub remember_me: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResendVerificationRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LogoutRequest {
    #[serde(default)]
    pub refresh_token: Option<String>,
}
