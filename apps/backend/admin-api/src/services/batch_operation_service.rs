use futures_util::future::join_all;
use ru5ty_gate_database::sqlx::Acquire;
use ru5ty_gate_database::{PgPool, sqlx};
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::{BatchOperationResult, BatchOperationSummary, RoleName};
use ru5ty_gate_utilities::PasswordUtil;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::schemas::{BulkCreateUserItem, BulkUpdateStatusItem};

#[derive(Clone)]
pub struct BatchOperationService {
    pool: PgPool,
}

fn success(id: String, data: Value) -> BatchOperationResult {
    BatchOperationResult {
        id,
        success: true,
        data: Some(data),
        error: None,
    }
}

fn failure(id: String, error: impl Into<String>) -> BatchOperationResult {
    BatchOperationResult {
        id,
        success: false,
        data: None,
        error: Some(error.into()),
    }
}

fn summarise(total: usize, results: Vec<BatchOperationResult>) -> BatchOperationSummary {
    let successful = results.iter().filter(|result| result.success).count();
    BatchOperationSummary {
        total,
        successful,
        failed: results.len() - successful,
        results,
    }
}

impl BatchOperationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn bulk_create_users(
        &self,
        users: &[BulkCreateUserItem],
        actor_id: Uuid,
    ) -> Result<BatchOperationSummary, AppError> {
        let role_id: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
            .bind(RoleName::ChatUser.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)?
            .ok_or_else(|| AppError::Internal("role 'Chat User' is missing".to_owned()))?;
        let status_id: Uuid =
            sqlx::query_scalar("SELECT id FROM user_statuses WHERE name = 'Pending Verification'")
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::internal)?
                .ok_or_else(|| {
                    AppError::Internal("user status 'Pending Verification' is missing".to_owned())
                })?;
        let hashes = join_all(users.iter().map(|user| PasswordUtil::hash(&user.password))).await;

        let mut results = Vec::with_capacity(users.len());
        for (index, (user, hash)) in users.iter().zip(hashes).enumerate() {
            let id = format!("user-{index}");
            let Some(hash) = hash else {
                results.push(failure(id, "Password hashing failed"));
                continue;
            };
            let inserted = sqlx::query_as::<_, (Uuid, String, String)>(
                "INSERT INTO users (username, email, password, ip_address, role_id, \
                 user_status_id, created_by, modified_by) \
                 VALUES ($1, $2, $3, '127.0.0.1', $4, $5, $6, $6) \
                 RETURNING id, email, username",
            )
            .bind(&user.name)
            .bind(&user.email)
            .bind(hash)
            .bind(role_id)
            .bind(status_id)
            .bind(actor_id.to_string())
            .fetch_one(&self.pool)
            .await;
            results.push(match inserted {
                Ok((created_id, email, username)) => success(
                    id,
                    json!({ "id": created_id, "email": email, "username": username }),
                ),
                Err(error)
                    if error
                        .as_database_error()
                        .is_some_and(|database| database.is_unique_violation()) =>
                {
                    failure(id, "Username or email already exists")
                }
                Err(error) => failure(id, error.to_string()),
            });
        }
        Ok(summarise(users.len(), results))
    }

    pub async fn bulk_update_user_status(
        &self,
        updates: &[BulkUpdateStatusItem],
        actor_id: Uuid,
    ) -> Result<BatchOperationSummary, AppError> {
        let mut transaction = self.pool.begin().await.map_err(AppError::internal)?;
        let mut results = Vec::with_capacity(updates.len());
        for update in updates {
            let id = update.user_id.to_string();
            let mut savepoint = transaction.begin().await.map_err(AppError::internal)?;
            let status_id: Option<Uuid> =
                sqlx::query_scalar("SELECT id FROM user_statuses WHERE name = $1")
                    .bind(&update.status)
                    .fetch_optional(&mut *savepoint)
                    .await
                    .map_err(AppError::internal)?;
            let Some(status_id) = status_id else {
                savepoint.rollback().await.map_err(AppError::internal)?;
                results.push(failure(
                    id,
                    format!("User status '{}' not found", update.status),
                ));
                continue;
            };
            let affected =
                sqlx::query("UPDATE users SET user_status_id = $2, modified_by = $3 WHERE id = $1")
                    .bind(update.user_id)
                    .bind(status_id)
                    .bind(actor_id.to_string())
                    .execute(&mut *savepoint)
                    .await
                    .map_err(AppError::internal)?;
            if affected.rows_affected() == 0 {
                savepoint.rollback().await.map_err(AppError::internal)?;
                results.push(failure(id, "User not found"));
            } else {
                savepoint.commit().await.map_err(AppError::internal)?;
                results.push(success(
                    id,
                    json!({ "userId": update.user_id, "status": update.status }),
                ));
            }
        }
        transaction.commit().await.map_err(AppError::internal)?;
        Ok(summarise(updates.len(), results))
    }

    pub async fn bulk_delete_users(
        &self,
        user_ids: &[Uuid],
        actor_id: Uuid,
    ) -> Result<BatchOperationSummary, AppError> {
        let mut seen = std::collections::HashSet::new();
        if !user_ids.iter().all(|id| seen.insert(*id)) {
            return Err(AppError::BadRequest(
                "Batch operation contains duplicate IDs".to_owned(),
            ));
        }
        let mut transaction = self.pool.begin().await.map_err(AppError::internal)?;
        let mut results = Vec::with_capacity(user_ids.len());
        for user_id in user_ids {
            let id = user_id.to_string();
            if *user_id == actor_id {
                return Err(AppError::BadRequest(format!(
                    "Batch transaction failed: Cannot delete your own account ({id})"
                )));
            }
            let deleted = sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(user_id)
                .execute(&mut *transaction)
                .await
                .map_err(AppError::internal)?;
            if deleted.rows_affected() == 0 {
                return Err(AppError::BadRequest(format!(
                    "Batch transaction failed: User not found ({id})"
                )));
            }
            results.push(success(id, json!({ "userId": user_id })));
        }
        transaction.commit().await.map_err(AppError::internal)?;
        Ok(summarise(user_ids.len(), results))
    }

    pub fn execute_custom_batch(&self, items: &[Map<String, Value>]) -> BatchOperationSummary {
        let results = items
            .iter()
            .enumerate()
            .map(|(index, item)| success(format!("item-{index}"), Value::Object(item.clone())))
            .collect();
        summarise(items.len(), results)
    }
}
