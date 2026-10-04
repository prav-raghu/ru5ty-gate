use ru5ty_gate_database::{PgPool, sqlx};
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::{ApiResponse, RoleName};
use ru5ty_gate_utilities::PasswordUtil;
use uuid::Uuid;

use crate::schemas::BootstrapAdminRequest;

const BOOTSTRAP_COST: u32 = 12;
const ALREADY_EXISTS: &str = "An administrator account already exists";

#[derive(Clone)]
pub struct BootstrapService {
    pool: PgPool,
    enabled: bool,
}

impl BootstrapService {
    pub fn new(pool: PgPool, enabled: bool) -> Self {
        Self { pool, enabled }
    }

    pub async fn bootstrap_admin(
        &self,
        request: &BootstrapAdminRequest,
    ) -> Result<ApiResponse<()>, AppError> {
        if !self.enabled {
            return Ok(ApiResponse::failure("Bootstrap is disabled"));
        }
        let password_hash = PasswordUtil::hash_with_cost(&request.password, BOOTSTRAP_COST)
            .await
            .ok_or_else(|| AppError::Internal("password hashing failed".to_owned()))?;

        let mut transaction = self.pool.begin().await.map_err(AppError::internal)?;
        sqlx::query(
            "INSERT INTO system_bootstrap (id) VALUES ('singleton') ON CONFLICT DO NOTHING",
        )
        .execute(&mut *transaction)
        .await
        .map_err(AppError::internal)?;
        let bootstrapped: bool = sqlx::query_scalar(
            "SELECT admin_bootstrapped FROM system_bootstrap WHERE id = 'singleton' FOR UPDATE",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(AppError::internal)?;
        if bootstrapped {
            return Ok(ApiResponse::failure(ALREADY_EXISTS));
        }

        let existing_admins: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users u JOIN roles r ON r.id = u.role_id WHERE r.name = $1",
        )
        .bind(RoleName::SuperAdmin.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(AppError::internal)?;
        if existing_admins > 0 {
            sqlx::query(
                "UPDATE system_bootstrap SET admin_bootstrapped = TRUE, bootstrapped_at = NOW() \
                 WHERE id = 'singleton'",
            )
            .execute(&mut *transaction)
            .await
            .map_err(AppError::internal)?;
            transaction.commit().await.map_err(AppError::internal)?;
            return Ok(ApiResponse::failure(ALREADY_EXISTS));
        }

        let role_id: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
            .bind(RoleName::SuperAdmin.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(AppError::internal)?
            .ok_or_else(|| AppError::Internal("role 'Super Admin' is missing".to_owned()))?;
        let status_id: Uuid =
            sqlx::query_scalar("SELECT id FROM user_statuses WHERE name = 'Online'")
                .fetch_optional(&mut *transaction)
                .await
                .map_err(AppError::internal)?
                .ok_or_else(|| AppError::Internal("user status 'Online' is missing".to_owned()))?;
        let created = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO users (username, email, password, ip_address, role_id, user_status_id) \
             VALUES ($1, $2, $3, '127.0.0.1', $4, $5) RETURNING id",
        )
        .bind(&request.username)
        .bind(&request.email)
        .bind(password_hash)
        .bind(role_id)
        .bind(status_id)
        .fetch_one(&mut *transaction)
        .await;
        let user_id = match created {
            Ok(id) => id,
            Err(error) => {
                if error
                    .as_database_error()
                    .is_some_and(|database| database.is_unique_violation())
                {
                    return Ok(ApiResponse::failure("Username or email is already in use"));
                }
                return Err(AppError::internal(error));
            }
        };
        sqlx::query(
            "UPDATE system_bootstrap SET admin_bootstrapped = TRUE, bootstrapped_at = NOW(), \
             bootstrapped_user_id = $1 WHERE id = 'singleton'",
        )
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(AppError::internal)?;
        transaction.commit().await.map_err(AppError::internal)?;
        tracing::warn!(%user_id, "Admin account bootstrapped - this route is now permanently locked");
        Ok(ApiResponse {
            is_successful: true,
            data: None,
            message: None,
            errors: None,
            date_time_stamp: None,
        })
    }
}
