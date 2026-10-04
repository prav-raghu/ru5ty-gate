---
name: feature-flags
description: Use when implementing feature flags — creating a new flag, evaluating flags in a service or frontend, setting up the DB-backed flag store, or integrating an external provider like Unleash or LaunchDarkly. Also use when a flag needs to be removed after a full rollout or cleaned up after a cancelled feature.
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

## Strategy — DB-backed by default

The template uses a Postgres-backed feature flag store by default: per-environment values via `APP_ENV`, per-user percentage rollouts, role-based targeting, no vendor costs. For large-scale production (100+ flags, real-time targeting, A/B analytics), migrate to Unleash (self-hosted) or LaunchDarkly by swapping the `FeatureFlagService` implementation — calling code doesn't change.

## Step 1 — Migration and model

A new migration in `common/database/migrations`:

```sql
CREATE TABLE feature_flags (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    key              VARCHAR(100) NOT NULL UNIQUE,
    description      VARCHAR(500) NOT NULL,
    is_enabled       BOOLEAN      NOT NULL DEFAULT FALSE,
    rollout_percent  INTEGER      NOT NULL DEFAULT 100 CHECK (rollout_percent BETWEEN 0 AND 100),
    allowed_roles    TEXT[]       NOT NULL DEFAULT '{}',
    allowed_user_ids TEXT[]       NOT NULL DEFAULT '{}',
    environments     TEXT[]       NOT NULL DEFAULT '{development,staging,production}',
    is_active        BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    created_by       VARCHAR(255) NOT NULL DEFAULT 'SYSTEM',
    modified_by      VARCHAR(255) NOT NULL DEFAULT 'SYSTEM'
);
CREATE INDEX idx_feature_flags_key_enabled ON feature_flags (key, is_enabled, is_active);
```

Add `models/feature_flag.rs` deriving `FromRow` and export it. Do not run the migration yourself.

## Step 2 — Flag constants (`common/types/src/feature_flag.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
    NewCheckoutFlow,
    EnhancedSearch,
    BulkExport,
}

impl FeatureFlag {
    pub fn key(self) -> &'static str {
        match self {
            Self::NewCheckoutFlow => "new-checkout-flow",
            Self::EnhancedSearch => "enhanced-search",
            Self::BulkExport => "bulk-export",
        }
    }
}
```

No magic strings in application code — always reference the enum. The exhaustive `match` makes the compiler enforce that every variant has a key.

## Step 3 — Feature flag service (`common/config` or a service-local `services/feature_flag_service.rs`)

```rust
#[derive(Debug, Clone, Default)]
pub struct FlagContext {
    pub user_id: Option<Uuid>,
    pub role: Option<String>,
}

#[derive(Clone)]
pub struct FeatureFlagService {
    pool: PgPool,
    cache: RedisService,
    environment: String,
}

impl FeatureFlagService {
    const CACHE_TTL_SECONDS: u64 = 60;

    pub async fn is_enabled(&self, flag: FeatureFlag, context: &FlagContext) -> bool {
        let Some(record) = self.load(flag).await else { return false };
        if !record.is_enabled || !record.environments.contains(&self.environment) {
            return false;
        }
        if let Some(user_id) = context.user_id
            && record.allowed_user_ids.contains(&user_id.to_string())
        {
            return true;
        }
        if let Some(role) = &context.role
            && record.allowed_roles.contains(role)
        {
            return true;
        }
        match context.user_id {
            Some(user_id) if record.rollout_percent < 100 => {
                bucket(flag, user_id) < u32::try_from(record.rollout_percent).unwrap_or(0)
            }
            Some(_) => true,
            None => record.rollout_percent == 100,
        }
    }

    async fn load(&self, flag: FeatureFlag) -> Option<FlagRecord> {
        let key = format!("feature-flag:{}", flag.key());
        if let Ok(Some(cached)) = self.cache.get_json::<FlagRecord>(&key).await {
            return Some(cached);
        }
        let record = sqlx::query_as::<_, FlagRecord>(
            "SELECT is_enabled, rollout_percent, allowed_roles, allowed_user_ids, environments \
             FROM feature_flags WHERE key = $1 AND is_active = TRUE",
        )
        .bind(flag.key())
        .fetch_optional(&self.pool)
        .await
        .ok()??;
        let _ = self.cache.set_json(&key, Self::CACHE_TTL_SECONDS, &record).await;
        Some(record)
    }
}

fn bucket(flag: FeatureFlag, user_id: Uuid) -> u32 {
    let digest = Sha256::digest(format!("{}:{user_id}", flag.key()));
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 100
}
```

The hash-based rollout is deterministic — the same user always lands in the same bucket, giving them a consistent experience across requests and sessions. `environment` comes from `ServiceConfig` (`"production"` or `"development"`); add `"staging"` handling only if the app config distinguishes it. A lookup or cache failure means the flag is off, never an error.

## Step 4 — Register in the service container

Add `FeatureFlagService` to `Services` in `types/services.rs` and construct it in `plugins/services.rs` for each backend service that needs it.

## Step 5 — Using flags in services

```rust
pub async fn checkout(&self, request: &CheckoutRequest, user: &AuthUser) -> Result<OrderDto, AppError> {
    let context = FlagContext { user_id: Some(user.id), role: Some(user.role.clone()) };
    if self.flags.is_enabled(FeatureFlag::NewCheckoutFlow, &context).await {
        self.new_checkout_flow(request, user.id).await
    } else {
        self.legacy_checkout_flow(request, user.id).await
    }
}
```

## Step 6 — Using flags in the frontend

Expose a public, aggressively cached `/feature-flags` endpoint in `customer-api`:

```rust
pub async fn get_flags(State(state): State<AppState>) -> Result<Response, AppError> {
    let flags = state.services.feature_flag.get_all().await?;
    Ok((StatusCode::OK, Json(ApiResponse::success(flags))).into_response())
}
```

`get_all` returns `HashMap<String, bool>` for active flags in the current environment.

```typescript
export function useFeatureFlags() {
  return useQuery({
    queryKey: ['feature-flags'],
    queryFn: async () => {
      const res = await apiClient.get<ResponseDto<Record<string, boolean>>>('/api/v1/feature-flags');
      return res.data.data ?? {};
    },
    staleTime: 1000 * 60,
    gcTime: 1000 * 60 * 5,
  });
}

export function useFlag(flag: string): boolean {
  const { data } = useFeatureFlags();
  return data?.[flag] ?? false;
}
```

## Step 7 — Managing flags via the admin API

| Method | Path | Permission | Purpose |
|--------|------|-----------|---------|
| GET | `/api/v1/feature-flags` | `Permission::SettingsRead` | List all flags |
| PUT | `/api/v1/feature-flags/{key}` | `Permission::SettingsWrite` | Enable/disable, adjust rollout |

Invalidate the flag's cache key (`feature-flag:{key}`) immediately when updated via the admin API, and set `modified_by` and `updated_at` in the update statement.

## Seed data

A migration or seed function with idempotent inserts:

```sql
INSERT INTO feature_flags (key, description, is_enabled, rollout_percent) VALUES
    ('new-checkout-flow', 'New checkout flow', FALSE, 0),
    ('enhanced-search', 'Enhanced search with full-text', FALSE, 0)
ON CONFLICT (key) DO NOTHING;
```

## Removing a flag after full rollout

1. Search the codebase for all `is_enabled(FeatureFlag::SomeFlag` usages
2. Remove the conditional, keep the new code path, delete the old one
3. Remove the variant from `FeatureFlag` in `common/types` (the compiler then finds every remaining use)
4. Add a migration to delete the row from `feature_flags`
5. Delete it from seed data

Never leave dead flags in the codebase — they accumulate into unreadable conditional forests.

## Tests

Service tests with `#[sqlx::test]` cover: disabled flag, wrong environment, allow-listed user and role, rollout bucket determinism (same user, same answer), 0% and 100% rollouts, and cache invalidation after an update.

## Critical rules

Never hardcode flag state (`if true`, `if false`) — always check `FeatureFlagService`. Never evaluate the same flag multiple times per request — call once, store the result locally. Never use flags for security gates — flags can be bypassed, use RBAC permissions for access control. Always reference flags through the `FeatureFlag` enum, never raw strings. Always seed flags with `is_enabled = FALSE` and `rollout_percent = 0` — opt-in, not opt-out.
