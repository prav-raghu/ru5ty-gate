use async_graphql::InputObject;
use serde::Serialize;

#[derive(Debug, Clone, InputObject, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: Option<bool>,
}
