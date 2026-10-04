use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::schemas::six_digit_code::six_digit_code;

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub remember_me: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VerifyLoginMfaRequest {
    #[validate(length(min = 1, max = 2048))]
    pub mfa_token: String,
    #[validate(custom(function = "six_digit_code"))]
    pub code: String,
    pub remember_me: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    #[serde(default)]
    #[validate(length(max = 2048))]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ForgotPasswordRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    pub token: Uuid,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    #[validate(length(min = 8, max = 128))]
    pub confirm_password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BootstrapAdminRequest {
    #[validate(length(min = 3, max = 30))]
    pub username: String,
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}
