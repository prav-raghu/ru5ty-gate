use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BulkCreateUserItem {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(min = 2, max = 30))]
    pub name: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BulkCreateUsersRequest {
    #[validate(length(min = 1, max = 500), nested)]
    pub users: Vec<BulkCreateUserItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BulkUpdateStatusItem {
    pub user_id: Uuid,
    #[validate(length(min = 1))]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BulkUpdateStatusRequest {
    #[validate(length(min = 1, max = 500), nested)]
    pub updates: Vec<BulkUpdateStatusItem>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BulkDeleteUsersRequest {
    #[validate(length(min = 1, max = 500))]
    pub user_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CustomBatchOperation {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CustomBatchRequest {
    pub operation: CustomBatchOperation,
    #[validate(length(min = 1, max = 100))]
    pub items: Vec<Map<String, Value>>,
}
