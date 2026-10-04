use ru5ty_gate_database::{PgPool, UserListFilter, UserRepository};
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::RoleName;
use uuid::Uuid;

use crate::dtos::{AuthorizedUser, ExportUser, UserSummary};

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Clone, Default)]
pub struct UserFilters {
    pub gender: Option<String>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_users(
        &self,
        filters: &UserFilters,
        logged_in_user_id: Option<Uuid>,
    ) -> Result<Vec<UserSummary>, AppError> {
        let filter = UserListFilter {
            exclude_user_id: logged_in_user_id,
            gender: filters.gender.as_deref(),
            min_age: filters.min_age,
            max_age: filters.max_age,
            limit: filters.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
            offset: filters.offset.unwrap_or(0).max(0),
        };
        let records = UserRepository::list_active(&self.pool, &filter)
            .await
            .map_err(AppError::internal)?;
        Ok(records.into_iter().map(UserSummary::from).collect())
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<UserSummary>, AppError> {
        let record = UserRepository::find_active_summary(&self.pool, user_id)
            .await
            .map_err(AppError::internal)?;
        Ok(record.map(UserSummary::from))
    }

    pub async fn get_authorized_user_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthorizedUser>, AppError> {
        let record =
            UserRepository::find_active_with_role(&self.pool, user_id, RoleName::ChatUser.as_str())
                .await
                .map_err(AppError::internal)?;
        Ok(record.map(AuthorizedUser::from))
    }

    pub async fn export_users(&self, offset: i64, limit: i64) -> Result<Vec<ExportUser>, AppError> {
        let records = UserRepository::list_for_export(&self.pool, offset, limit)
            .await
            .map_err(AppError::internal)?;
        Ok(records.into_iter().map(ExportUser::from).collect())
    }
}
