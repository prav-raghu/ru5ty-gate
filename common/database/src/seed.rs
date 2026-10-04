use ru5ty_gate_types::{ADMIN_TIER_ROLES, CUSTOMER_TIER_ROLES, RoleName};
use sqlx::PgPool;

use crate::database_error::DatabaseError;

pub const USER_STATUS_NAMES: [&str; 4] = ["Online", "Offline", "Pending Verification", "Verified"];

const BCRYPT_COST: u32 = 10;

#[derive(Debug, Clone)]
pub struct SeedAdmin {
    pub email: String,
    pub username: String,
    pub password: String,
}

pub async fn seed_roles(pool: &PgPool) -> Result<(), DatabaseError> {
    let names = ADMIN_TIER_ROLES
        .iter()
        .chain(CUSTOMER_TIER_ROLES.iter())
        .map(|role| role.as_str());
    for name in names {
        sqlx::query("INSERT INTO roles (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
            .bind(name)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn seed_user_statuses(pool: &PgPool) -> Result<(), DatabaseError> {
    for name in USER_STATUS_NAMES {
        sqlx::query("INSERT INTO user_statuses (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
            .bind(name)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn seed_admin(pool: &PgPool, admin: &SeedAdmin) -> Result<bool, DatabaseError> {
    let existing_admins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users u JOIN roles r ON r.id = u.role_id WHERE r.name = $1",
    )
    .bind(RoleName::SuperAdmin.as_str())
    .fetch_one(pool)
    .await?;
    if existing_admins > 0 {
        return Ok(false);
    }
    let email_taken: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
            .bind(&admin.email)
            .fetch_one(pool)
            .await?;
    if email_taken {
        return Ok(false);
    }

    let status_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM user_statuses WHERE name = $1")
        .bind("Online")
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DatabaseError::SeedPrerequisite("user status 'Online'".to_owned()))?;
    let role_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
        .bind(RoleName::SuperAdmin.as_str())
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| DatabaseError::SeedPrerequisite("role 'Super Admin'".to_owned()))?;
    let password_hash = bcrypt::hash(&admin.password, BCRYPT_COST)?;

    sqlx::query(
        "INSERT INTO users (username, email, password, ip_address, user_status_id, role_id) \
         VALUES ($1, $2, $3, '127.0.0.1', $4, $5)",
    )
    .bind(&admin.username)
    .bind(&admin.email)
    .bind(password_hash)
    .bind(status_id)
    .bind(role_id)
    .execute(pool)
    .await?;
    Ok(true)
}

pub async fn seed_all(pool: &PgPool, admin: Option<&SeedAdmin>) -> Result<(), DatabaseError> {
    seed_roles(pool).await?;
    seed_user_statuses(pool).await?;
    if let Some(admin) = admin {
        seed_admin(pool, admin).await?;
    }
    Ok(())
}
