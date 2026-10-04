use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "user:read")]
    UserRead,
    #[serde(rename = "user:write")]
    UserWrite,
    #[serde(rename = "user:delete")]
    UserDelete,
    #[serde(rename = "role:read")]
    RoleRead,
    #[serde(rename = "role:assign")]
    RoleAssign,
    #[serde(rename = "report:view")]
    ReportView,
    #[serde(rename = "report:export")]
    ReportExport,
    #[serde(rename = "settings:read")]
    SettingsRead,
    #[serde(rename = "settings:write")]
    SettingsWrite,
    #[serde(rename = "batch:write")]
    BatchWrite,
    #[serde(rename = "venue:read")]
    VenueRead,
    #[serde(rename = "venue:write")]
    VenueWrite,
}

impl Permission {
    pub const ALL: [Permission; 12] = [
        Self::UserRead,
        Self::UserWrite,
        Self::UserDelete,
        Self::RoleRead,
        Self::RoleAssign,
        Self::ReportView,
        Self::ReportExport,
        Self::SettingsRead,
        Self::SettingsWrite,
        Self::BatchWrite,
        Self::VenueRead,
        Self::VenueWrite,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserRead => "user:read",
            Self::UserWrite => "user:write",
            Self::UserDelete => "user:delete",
            Self::RoleRead => "role:read",
            Self::RoleAssign => "role:assign",
            Self::ReportView => "report:view",
            Self::ReportExport => "report:export",
            Self::SettingsRead => "settings:read",
            Self::SettingsWrite => "settings:write",
            Self::BatchWrite => "batch:write",
            Self::VenueRead => "venue:read",
            Self::VenueWrite => "venue:write",
        }
    }
}
