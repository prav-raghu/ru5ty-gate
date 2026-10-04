use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserListQuery {
    pub gender: Option<String>,
    pub min_age: Option<u32>,
    pub max_age: Option<u32>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPath {
    pub user_id: Uuid,
}
