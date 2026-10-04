use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoleName {
    #[serde(rename = "Chat User")]
    ChatUser,
    #[serde(rename = "Super Admin")]
    SuperAdmin,
    #[serde(rename = "Moderator")]
    Moderator,
    #[serde(rename = "Support")]
    Support,
}

pub const ADMIN_TIER_ROLES: [RoleName; 3] =
    [RoleName::SuperAdmin, RoleName::Moderator, RoleName::Support];

pub const CUSTOMER_TIER_ROLES: [RoleName; 1] = [RoleName::ChatUser];

impl RoleName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ChatUser => "Chat User",
            Self::SuperAdmin => "Super Admin",
            Self::Moderator => "Moderator",
            Self::Support => "Support",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Chat User" => Some(Self::ChatUser),
            "Super Admin" => Some(Self::SuperAdmin),
            "Moderator" => Some(Self::Moderator),
            "Support" => Some(Self::Support),
            _ => None,
        }
    }

    pub fn is_admin_tier(self) -> bool {
        ADMIN_TIER_ROLES.contains(&self)
    }

    pub fn is_customer_tier(self) -> bool {
        CUSTOMER_TIER_ROLES.contains(&self)
    }
}
