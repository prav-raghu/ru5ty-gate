use async_graphql::{ID, SimpleObject};
use serde_json::Value;

#[derive(Debug, Clone, SimpleObject)]
pub struct User {
    pub id: ID,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn text(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
}

impl User {
    pub fn from_json(value: &Value) -> Option<Self> {
        let id = value.get("id").and_then(Value::as_str)?;
        let created = text(value, &["createdAt", "lastSeen"]).unwrap_or_default();
        Some(Self {
            id: ID(id.to_owned()),
            email: text(value, &["email"]).unwrap_or_default(),
            first_name: text(value, &["firstName", "username"]),
            last_name: text(value, &["lastName"]),
            role: text(value, &["role"]).unwrap_or_else(|| "Chat User".to_owned()),
            is_active: value
                .get("isActive")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            updated_at: text(value, &["updatedAt"]).unwrap_or_else(|| created.clone()),
            created_at: created,
        })
    }
}
