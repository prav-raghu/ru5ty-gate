use async_graphql::SimpleObject;

use crate::graphql::schemas::User;

#[derive(Debug, Clone, SimpleObject)]
pub struct UserResponse {
    pub success: bool,
    pub data: Option<User>,
    pub error: Option<String>,
}
