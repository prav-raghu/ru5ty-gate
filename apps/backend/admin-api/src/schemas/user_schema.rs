use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::schemas::six_digit_code::six_digit_code;

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OnboardingRequest {
    #[validate(length(min = 3, max = 30))]
    pub username: String,
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub gender: Option<String>,
    #[validate(range(min = 0, max = 120))]
    pub age: Option<i32>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub allow_email_communications: bool,
    pub accept_terms_and_conditions: Option<bool>,
    pub ip_address: String,
    pub user_status_id: Uuid,
    pub role_id: Uuid,
    pub join_date: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct ResendVerificationRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileRequest {
    #[validate(length(min = 3, max = 30))]
    pub username: String,
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(max = 500))]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, max = 128))]
    pub current_password: String,
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
    #[validate(length(min = 8, max = 128))]
    pub confirm_password: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Verify2FaRequest {
    #[validate(custom(function = "six_digit_code"))]
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct Disable2FaRequest {
    #[validate(custom(function = "six_digit_code"))]
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailPath {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsernamePath {
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserIdPath {
    pub user_id: Uuid,
}
