use async_graphql::SimpleObject;

use crate::graphql::schemas::User;

#[derive(Debug, Clone, SimpleObject)]
pub struct UsersResponse {
    pub success: bool,
    pub data: Option<Vec<User>>,
    pub error: Option<String>,
}
