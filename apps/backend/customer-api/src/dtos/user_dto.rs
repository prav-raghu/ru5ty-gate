use chrono::{DateTime, Utc};
use ru5ty_gate_database::{AuthorizedUserRecord, ExportUserRecord, UserSummaryRecord};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub age: Option<i32>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuthorizedUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role_name: String,
}

#[derive(Debug, Clone)]
pub struct ExportUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

impl From<UserSummaryRecord> for UserSummary {
    fn from(record: UserSummaryRecord) -> Self {
        Self {
            id: record.id,
            username: record.username,
            age: record.age,
            last_seen: record.last_seen,
        }
    }
}

impl From<AuthorizedUserRecord> for AuthorizedUser {
    fn from(record: AuthorizedUserRecord) -> Self {
        Self {
            id: record.id,
            username: record.username,
            email: record.email,
            role_name: record.role_name,
        }
    }
}

impl From<ExportUserRecord> for ExportUser {
    fn from(record: ExportUserRecord) -> Self {
        Self {
            id: record.id,
            email: record.email,
            username: record.username,
            created_at: record.created_at,
        }
    }
}
