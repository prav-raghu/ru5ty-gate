---
name: audit-log
description: Use when implementing audit trails for state-changing operations — adding audit logging to a service, deciding what to audit, querying audit history, or setting up retention policy. Trigger on "audit log", "track who changed this", or "log before/after values".
tools: Read, Edit, Write, Grep, Glob
model: inherit
---

## When to audit

Not every table needs auditing. Audit state changes where knowing who changed what and when has regulatory, legal, or operational value: user accounts (role changes, deactivations), payment and order records, permissions and role assignments, settings/configuration, and anything explicitly marked auditable in requirements. Never audit read operations or log-noise tables (`webhook_deliveries`, queue jobs).

## Table

A new migration in `common/database/migrations`:

```sql
CREATE TABLE audit_logs (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    entity      VARCHAR(100) NOT NULL,
    entity_id   VARCHAR(255) NOT NULL,
    action      VARCHAR(50)  NOT NULL,
    before      JSONB,
    after       JSONB,
    changed_by  VARCHAR(255) NOT NULL,
    ip_address  VARCHAR(45),
    user_agent  VARCHAR(500),
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_audit_logs_entity ON audit_logs (entity, entity_id);
CREATE INDEX idx_audit_logs_changed_by ON audit_logs (changed_by);
CREATE INDEX idx_audit_logs_created_at ON audit_logs (created_at);
```

No `updated_at`, no `is_active` — audit logs are immutable, append-only (the append-only exception in `rules/database.md`).

## AuditAction (`common/types/src/audit_action.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Restore,
    RoleAssign,
    PasswordChange,
    Login,
    Logout,
}

#[derive(Debug, Clone)]
pub struct AuditContext {
    pub user_id: Uuid,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
```

Each type lives in its own file and is re-exported from `lib.rs`.

## AuditLogService (`services/audit_log_service.rs`)

```rust
#[derive(Clone)]
pub struct AuditLogService {
    pool: PgPool,
}

impl AuditLogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn log(
        &self,
        entity: &str,
        entity_id: &str,
        action: AuditAction,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
        context: &AuditContext,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO audit_logs (entity, entity_id, action, before, after, changed_by, ip_address, user_agent) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(entity)
        .bind(entity_id)
        .bind(action.as_str())
        .bind(before)
        .bind(after.map(redact))
        .bind(context.user_id.to_string())
        .bind(context.ip_address.as_deref())
        .bind(context.user_agent.as_deref().map(|agent| agent.chars().take(500).collect::<String>()))
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(())
    }
}

fn redact(mut value: serde_json::Value) -> serde_json::Value {
    const SENSITIVE: [&str; 5] = ["password", "token", "secret", "hash", "two_factor_secret"];
    if let Some(object) = value.as_object_mut() {
        for key in SENSITIVE {
            if object.contains_key(key) {
                object.insert(key.to_owned(), serde_json::Value::String("[REDACTED]".to_owned()));
            }
        }
    }
    value
}
```

`redact` uses a fixed list of exact keys; extend the list when a new entity carries other secrets. Pass `before` through `redact` as well when it can contain sensitive fields.

## Using it in a domain service

Call `log()` AFTER the database write succeeds.

```rust
pub async fn update(&self, id: Uuid, request: &UpdateUserRequest, context: &AuditContext) -> Result<UserDto, AppError> {
    let existing = self.find_active(id).await?;
    let updated = self.write_update(id, request, context.user_id).await?;
    self.cache.del(&format!("user:{id}")).await.ok();
    self.audit
        .log(
            "user",
            &id.to_string(),
            AuditAction::Update,
            Some(json!({ "email": existing.email, "roleId": existing.role_id })),
            Some(json!({ "email": updated.email, "roleId": updated.role_id })),
            context,
        )
        .await?;
    Ok(updated)
}
```

## Passing AuditContext from the controller

```rust
pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ApiPath(path): ApiPath<UserPath>,
    ValidatedJson(request): ValidatedJson<UpdateUserRequest>,
) -> Result<Response, AppError> {
    let context = AuditContext {
        user_id: user.id,
        ip_address: Some(ip),
        user_agent: headers.get(USER_AGENT).and_then(|value| value.to_str().ok()).map(str::to_owned),
    };
    let updated = state.services.user.update(path.user_id, &request, &context).await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(updated))).into_response())
}
```

## What goes in before/after

Only the fields that could actually change — not the entire row. Keeps payloads small and diffs readable. `before` is `None` for `create` actions, `after` is `None` for `delete` actions.

## Querying audit history

Add a read-only endpoint in admin-api: `GET /api/v1/audit-logs?entity=user&entityId={id}&page=1&pageSize=50`, in the protected router behind a dedicated `Permission::AuditRead` (see the `rbac` agent).

## Retention

`audit_logs` grows indefinitely. Add a scheduled cleanup job in `schedule-api` deleting entries older than the retention policy (e.g. 2 years for regulated industries, 90 days standard). Never manually delete audit entries outside the scheduled job.

## Rules

Never log before the database write — if the write fails, the audit entry shouldn't exist. Never audit read operations. Always redact sensitive fields (`password`, `token`, `secret`) before writing. Never write audit entries inside the same transaction as the entity write — a rollback would delete the audit too; keep them separate. `before`/`after` store only the changed-field subset, never the full row.
