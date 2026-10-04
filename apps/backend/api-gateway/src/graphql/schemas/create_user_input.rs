use async_graphql::InputObject;
use serde::Serialize;

#[derive(Debug, Clone, InputObject, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub email: String,
    pub password: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
}
