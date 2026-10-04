---
name: rbac
description: Use when implementing role-based access control — adding permissions to routes, defining role-to-permission mappings, creating permission guards, or restricting service methods to specific roles. Also use when a new role or permission needs to be added to the system, or when auditing which endpoints are accessible to which roles.
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

## Overview

RBAC is built on three layers: permissions (fine-grained `entity:action` values), role-to-permission mapping (defined in `common/types`), and a permission layer (the `require_permission` middleware in `common/http` enforcing permissions per route).

## Creating the first SUPER_ADMIN

Never create the first admin account through the normal user-creation path with the guard disabled or bypassed. There is exactly one sanctioned way in — `admin-api`'s one-time, self-locking `/auth/bootstrap-admin` route, documented in `jwt-security.md`'s "Admin bootstrap" section. It only fires when no admin has ever been bootstrapped (tracked by the persisted `system_bootstrap` row, not by counting current admins), and it permanently refuses after the first success — deleting or demoting the bootstrapped admin later does not reopen it.

## Support starts read-only

`Support` is deliberately scoped to read permissions only at launch — no write, manage, delete, export, or assign. This is the default for a new project, not a limitation to work around. Only add write-shaped permissions to `RoleName::Support` in `get_permissions_for_role` when the actual product scope calls for it (e.g. "Support needs to log tickets" → add a `TicketWrite` permission once a ticketing domain exists). Elevate one permission at a time, deliberately, in the same PR that introduces the feature it is for — never grant `Support` broad write access preemptively.

## Existing roles (`common/types/src/role_name.rs`)

| Role | Tier | Description |
|------|------|-------------|
| `RoleName::SuperAdmin` | Admin | Full system access |
| `RoleName::Moderator` | Admin | Manage users, reports, batch operations |
| `RoleName::Support` | Admin | Read-only by default |
| `RoleName::ChatUser` | Customer | Customer-facing features only |

Roles serialise to their display names (`"Super Admin"`, `"Chat User"`, ...) because the `roles` table stores those names. `ADMIN_TIER_ROLES` and `CUSTOMER_TIER_ROLES` define the tiers.

## Step 1 — Define permissions (`common/types/src/permission.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "user:read")]
    UserRead,
    #[serde(rename = "user:write")]
    UserWrite,
    #[serde(rename = "product:read")]
    ProductRead,
    #[serde(rename = "product:write")]
    ProductWrite,
}

impl Permission {
    pub const ALL: [Permission; 4] = [Self::UserRead, Self::UserWrite, Self::ProductRead, Self::ProductWrite];

    pub fn as_str(self) -> &'static str { ... }
}
```

The serde rename is the wire format and goes into the JWT. Keep `ALL` and `as_str` in sync with the enum — `super_admin_has_every_permission` in `common/types/tests/types.rs` fails when `ALL` drifts from what `SuperAdmin` receives.

## Step 2 — Role-permission map (`common/types/src/rbac.rs`)

```rust
pub fn get_permissions_for_role(role: RoleName) -> Vec<Permission> {
    match role {
        RoleName::SuperAdmin => Permission::ALL.to_vec(),
        RoleName::Moderator => vec![Permission::UserRead, Permission::UserWrite, Permission::ProductRead, Permission::ProductWrite],
        RoleName::Support => vec![Permission::UserRead, Permission::ProductRead],
        RoleName::ChatUser => vec![Permission::ProductRead],
    }
}

pub fn role_has_permission(role: RoleName, permission: Permission) -> bool {
    get_permissions_for_role(role).contains(&permission)
}
```

The match is exhaustive on purpose: adding a `RoleName` variant fails compilation until its permissions are decided. Re-export both functions from `common/types/src/lib.rs`.

## Step 3 — Permissions travel in the JWT

`TokenService::generate_token` resolves `get_permissions_for_role` at sign time and embeds `permissions` and `scope` in the access token, so the guard checks without a database lookup per request. `authenticate` copies them into `AuthUser { id, username, email, role, permissions, scope }` in the request extensions.

## Step 4 — Permission layer (`common/http/src/middleware/auth.rs`)

```rust
pub async fn require_permission(
    State(required): State<Permission>,
    request: Request,
    next: Next,
) -> Response
```

It reads `AuthUser` from the extensions and returns 403 with the standard envelope when `required` is not in `user.permissions`. It must only run inside a router that already has the `authenticate` layer, so an unauthenticated request gets 401 first.

## Step 5 — Protect routes

```rust
pub fn protected() -> Router<AppState> {
    let permission = |required: Permission| from_fn_with_state(required, require_permission);
    let reads = Router::new()
        .route("/users/{userId}/details", get(UserController::get_user_details))
        .route_layer(permission(Permission::UserRead));
    let writes = Router::new()
        .route("/users/onboarding", post(UserController::onboard_user))
        .route_layer(permission(Permission::UserWrite));
    Router::new().merge(reads).merge(writes)
}
```

Group routes by permission and apply one `route_layer` per group, as `admin-api`'s `user_route.rs` does. Public routes live in `public()` routers that never pass through `authenticate`.

## Permission naming convention

`{entity}:{action}` — `read` (list + get, safe), `write` (create + update), `delete` (soft delete), `manage` (elevated write — approve/reject/escalate), `export` (download/bulk export), `assign` (assign to another entity, e.g. role to user), `view` (reports).

## Adding a new permission

1. Add the variant (with its `#[serde(rename = "entity:action")]`) to `Permission`, add it to `ALL` and to `as_str`
2. Add it to the appropriate roles in `get_permissions_for_role`
3. Apply it with `require_permission` on the relevant route group
4. Add a test in `common/types` and a 403/200 route case in the service's `tests/integration`

## Adding a new role

1. Add the variant to `RoleName` with its `#[serde(rename = "...")]` display name, and to `as_str` and `from_name`
2. Add it to `ADMIN_TIER_ROLES` or `CUSTOMER_TIER_ROLES` as appropriate
3. Add its arm in `get_permissions_for_role` (the compiler enforces this)
4. Insert the role in a new migration (and the seed functions) so the `roles` table has the row

## Critical rules

Never check roles directly in service methods — only check permissions, and only through `require_permission`. Never put role/permission logic inside controllers. Public routers skip all auth — only for truly unauthenticated endpoints. `SuperAdmin` always gets `Permission::ALL` — never list its permissions manually. Permissions are embedded in the JWT at login time — changing `get_permissions_for_role` requires a re-login (or a logout, which invalidates old tokens) to take effect. Token scope (`Customer` vs `Admin`) is enforced by each service's authenticator, so a customer token can never reach admin routes even if its permissions overlap.
