use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::dtos::{CurrentUserRecord, UserDetailsRecord};

#[derive(Debug, Clone, Serialize)]
pub struct NamedItem {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserDto {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub allow_email_communications: bool,
    pub two_factor_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub roles: NamedItem,
    pub status: NamedItem,
}

impl From<CurrentUserRecord> for CurrentUserDto {
    fn from(record: CurrentUserRecord) -> Self {
        Self {
            id: record.id,
            username: record.username,
            email: record.email,
            avatar: record.avatar,
            allow_email_communications: record.allow_email_communications,
            two_factor_enabled: record.two_factor_enabled,
            created_at: record.created_at,
            last_seen: record.last_seen,
            roles: NamedItem {
                id: record.role_id,
                name: record.role_name,
            },
            status: NamedItem {
                id: record.status_id,
                name: record.status_name,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailabilityByEmail {
    pub email: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailabilityByUsername {
    pub username: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileData {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup2FaData {
    pub secret: String,
    pub qr_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupDetails {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub modified_by: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailsData {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub avatar: Option<String>,
    pub gender_id: Option<String>,
    pub age: Option<i32>,
    pub accept_terms_and_conditions: bool,
    pub allow_email_communications: bool,
    pub ip_address: String,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
    pub two_factor_enabled: bool,
    pub user_status_id: Uuid,
    pub role_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub modified_by: String,
    pub status: LookupDetails,
    pub roles: LookupDetails,
    pub ip_addresses: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDetailsDto {
    pub is_successful: bool,
    pub message: String,
    pub data: UserDetailsData,
}

impl From<UserDetailsRecord> for UserDetailsDto {
    fn from(record: UserDetailsRecord) -> Self {
        let status = LookupDetails {
            id: record.user_status_id,
            name: record.status_name,
            is_active: record.status_is_active,
            created_at: record.status_created_at,
            updated_at: record.status_updated_at,
            created_by: record.status_created_by,
            modified_by: record.status_modified_by,
        };
        let roles = LookupDetails {
            id: record.role_id,
            name: record.role_name,
            is_active: record.role_is_active,
            created_at: record.role_created_at,
            updated_at: record.role_updated_at,
            created_by: record.role_created_by,
            modified_by: record.role_modified_by,
        };
        Self {
            is_successful: true,
            message: "User details retrieved successfully".to_owned(),
            data: UserDetailsData {
                id: record.id,
                username: record.username,
                email: record.email,
                avatar: record.avatar,
                gender_id: record.gender,
                age: record.age,
                accept_terms_and_conditions: record.accept_terms_and_conditions,
                allow_email_communications: record.allow_email_communications,
                ip_addresses: vec![record.ip_address.clone()],
                ip_address: record.ip_address,
                last_seen: record.last_seen,
                is_active: record.is_active,
                two_factor_enabled: record.two_factor_enabled,
                user_status_id: record.user_status_id,
                role_id: record.role_id,
                created_at: record.created_at,
                updated_at: record.updated_at,
                created_by: record.created_by,
                modified_by: record.modified_by,
                status,
                roles,
            },
        }
    }
}
