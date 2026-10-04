use crate::permission::Permission;
use crate::role_name::RoleName;

pub fn get_permissions_for_role(role: RoleName) -> Vec<Permission> {
    match role {
        RoleName::SuperAdmin => Permission::ALL.to_vec(),
        RoleName::Moderator => vec![
            Permission::UserRead,
            Permission::UserWrite,
            Permission::RoleRead,
            Permission::ReportView,
            Permission::ReportExport,
            Permission::BatchWrite,
            Permission::VenueRead,
        ],
        RoleName::Support => vec![
            Permission::UserRead,
            Permission::RoleRead,
            Permission::ReportView,
            Permission::VenueRead,
        ],
        RoleName::ChatUser => vec![Permission::UserRead],
    }
}

pub fn role_has_permission(role: RoleName, permission: Permission) -> bool {
    get_permissions_for_role(role).contains(&permission)
}
