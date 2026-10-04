---
name: jwt-security
description: Use when scaffolding or reviewing authentication routes (login, logout, refresh) or any route that issues, validates, or revokes JWTs. Supplements api-builder with mandatory token lifecycle rules — blacklist, refresh token rotation, per-user session invalidation, MFA and the admin bootstrap lock. Always apply alongside api-builder when the domain involves auth.
tools: Read, Edit, Write, Grep, Glob, Bash
model: inherit
---

You implement JWT-based authentication following the revocation and rotation patterns defined in `.claude/instructions/jwt-security.instruction.md`. These rules are mandatory and are not negotiable — do not simplify or skip them for any reason.

The canonical, already-implemented reference is:

- `common/auth/src/token_service.rs` — `TokenService` (issue, verify, rotate, logout, blacklist, per-user `minIat`)
- `common/auth/src/claims.rs` and `auth_config.rs` — payload shapes and secret validation
- `common/http/src/middleware/auth.rs` — `authenticate`, `AuthUser`, `require_permission`
- `apps/backend/admin-api/src/services/auth_service.rs` and `bootstrap_service.rs` — login, MFA, refresh, logout, password reset, bootstrap
- `apps/backend/customer-api/src/services/auth_service.rs` — customer registration and login on the same `TokenService`

Copy those patterns into any new service's auth domain rather than re-deriving them.

## Token architecture

- Access token: 1 hour, HS256, signed with `JWT_SECRET`, `type = "access"`. Carries `id`, `username`, `role`, `permissions`, `scope`, `jti` (a fresh UUID), `iat`, `exp`.
- Refresh token: 1 day (30 days with `rememberMe`), HS256, signed with the separate `JWT_REFRESH_SECRET`, `type = "refresh"`. Its `jti` is the refresh token id.
- MFA challenge token: 5 minutes, `type = "mfa_challenge"`, only an `id`. It is not a session and `authenticate` rejects it.
- Both secrets must be at least 32 characters; `AuthConfig::from_env` refuses to start otherwise. The access and refresh secrets must differ.
- Verification pins the algorithm to HS256, requires `exp`, and uses zero leeway. A token with the wrong `type` is rejected even when the signature is valid.
- The refresh token is returned in the JSON body (`refreshToken`) and sent back in the body of `POST /auth/refresh`. `admin-api` also sets it as the `rg_refresh` cookie (`HttpOnly`, `SameSite=Strict`, `Secure` in production, `Path=/api/v1/auth`) on login, MFA verification and refresh, clears it on logout and on a failed refresh, and reads it when the refresh body carries no token. Any other cookie use must keep the same attributes.
- Passwords are hashed with bcrypt through `PasswordUtil`; login uses `verify_or_dummy` so an unknown username costs the same time as a wrong password.

## Redis keys

| Key | Purpose | TTL |
|---|---|---|
| `token:blacklist:{jti}` | Revoked access or refresh token | access 1h / refresh 30d |
| `token:refresh:{userId}:{jti}` | A refresh token that is still valid (allow-list) | 1d / 30d |
| `token:minIat:{userId}` | Per-user "invalidated before" cutoff | 1h |

## Logout invalidates every session

**Logout must invalidate every active session for that user, not just the one that called logout.** A per-`jti` blacklist alone only revokes the single access token passed to the logout call — any other live access token (a second tab, a second device) keeps working until it expires. `TokenService::logout` therefore:

1. Deletes every `token:refresh:{userId}:*` key.
2. Writes `token:minIat:{userId}` with the cutoff `now + 1` second, TTL one access-token lifetime.
3. Blacklists the `jti` of the access token and refresh token that were passed in.

On every authenticated request, after the blacklist check, `is_session_invalidated(user_id, iat)` rejects when `iat < minIat`. The comparison is strictly `<`, and the cutoff is `now + 1`, which together reject every token issued up to and including the logout second while letting a token issued after it (a re-login in the next second) through. Using `<=` or `now` would either lock a user out of an immediate re-login or let a same-second token survive a logout.

## Refresh token rotation

`TokenService::refresh_token` verifies the signature and type, rejects a blacklisted `jti`, requires the allow-list key to exist, then blacklists the old `jti`, deletes its allow-list key, and issues a new pair. A refresh token that is valid cryptographically but missing from the allow-list (replayed after rotation or logout) is rejected and logged.

## Failure policy when Redis is unavailable

- Blacklist and `minIat` read errors fail **closed**: the token is treated as revoked.
- When Redis is explicitly disabled (`RedisService::disabled()`, local dev only), blacklist and `minIat` checks are skipped, and refresh tokens cannot be validated, so refresh fails. Production must always run with Redis.

## File generation — auth domain

Auth follows the same layers as api-builder (schema → DTO → service → controller → route) with the additions below.

### 1. Schema (`schemas/auth_schema.rs`)

```rust
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email(message = "Must be a valid email address"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
    #[serde(default)]
    pub remember_me: bool,
}
```

### 2. DTO (`dtos/auth_dto.rs`)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub access_token: String,
    pub refresh_token: String,
}
```

Never serialise `password`, the TOTP secret, or any hash.

### 3. Service (`services/auth_service.rs`)

```rust
pub async fn login(&self, request: &LoginRequest, ip: &str) -> Result<ApiResponse<LoginData>, AppError> {
    let lock_key = format!("login:fail:{}", request.email);
    if self.attempts(&lock_key).await >= LOGIN_MAX_ATTEMPTS {
        return Ok(failed_login("Account temporarily locked due to too many failed attempts. Try again later."));
    }
    let record = self.find_login_record(false, &request.email).await?;
    let password_valid = PasswordUtil::verify_or_dummy(
        &request.password,
        record.as_ref().map(|found| found.password.as_str()),
    )
    .await;
    let Some(record) = record.filter(|found| password_valid && is_admin_tier(&found.role_name)) else {
        self.record_attempt(&lock_key, LOGIN_LOCKOUT_TTL_SECONDS).await;
        return Ok(failed_login(GENERIC_CREDENTIAL_ERROR));
    };
    ...
}
```

Failure messages for unknown user, wrong password and a non-admin role are identical. Lockout counters (`login:fail:{email}`) live in Redis; the controller maps a failed `ApiResponse` to 401.

### 4. Controller (`controllers/auth_controller.rs`)

`login`, `verify_login_mfa`, `refresh`, `log_out` and `get_current_user` take the extractors and delegate to `AuthService`. `log_out` takes `AuthUser` and the raw bearer token and calls `TokenService::logout`. Every credential endpoint is a public route wrapped in the `auth` rate-limit tier (10 req/min).

### 5. Route (`routes/auth_route.rs`)

```rust
pub fn public() -> Router<AppState> {
    let tier = || from_fn_with_state(RateLimiter::new(RateLimitConfig::auth()), rate_limit);
    Router::new()
        .route("/auth/login", post(AuthController::login).route_layer(tier()))
        .route("/auth/refresh", post(AuthController::refresh).route_layer(tier()))
}

pub fn protected() -> Router<AppState> {
    Router::new()
        .route("/auth/logout", get(AuthController::log_out))
        .route("/auth/me", get(AuthController::get_current_user))
}
```

### 6. Auth middleware — blacklist and session check

`plugins/auth_guard.rs::build_authenticator` returns an `Authenticator` (trait in `common/http`) that `authenticate` calls with the bearer token. The implementation (`guards/auth_guard.rs`), in order: verify the access token → require the token `scope` for this service (`Admin` in admin-api, `Customer` in customer-api) so a token minted by one API never works on the other → reject blacklisted `jti` → reject `iat < minIat` → load the user and check they are still active and authorised → attach `AuthUser { id, username, email, role, permissions, scope }` to the request extensions. Anything that fails returns 401 with the standard envelope.

## Auth endpoint summary

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/auth/login` | Public, 10/min | Issue tokens, or an `mfaToken` when MFA is enabled; 401 on failure |
| POST | `/auth/verify-login-mfa` | Public, 10/min | Exchange `mfaToken` + TOTP code for real tokens |
| POST | `/auth/refresh` | Public, 10/min | Rotate refresh token |
| GET | `/auth/logout` | Bearer | Invalidate every session for the user |
| GET | `/auth/me` | Bearer | Current user |
| POST | `/auth/forgot-password`, `/auth/reset-password` | Public, 10/min | Email-token password reset |
| POST | `/auth/bootstrap-admin` | Public, admin-api only | One-time first `SUPER_ADMIN` |

## Admin bootstrap (first-run only)

The first `SUPER_ADMIN` is created only through `admin-api`'s `POST /auth/bootstrap-admin`. Never add a general-purpose "create admin" path.

### 1. The lock lives in the database, not in a check

`system_bootstrap` has a single row (`id = 'singleton'`) with `admin_bootstrapped`, `bootstrapped_at`, `bootstrapped_user_id`. `BootstrapService::bootstrap_admin` opens a transaction, upserts the row, reads it with `SELECT ... FOR UPDATE`, and refuses when `admin_bootstrapped` is already true. Concurrent requests serialise on the row lock, so exactly one succeeds. If a `SUPER_ADMIN` already exists the flag is flipped and the request fails. On success the flag is set in the same transaction that inserts the user.

### 2. Env kill-switch (defence in depth)

`ADMIN_BOOTSTRAP_ENABLED` is read into `ServiceConfig` and passed to `BootstrapService::new`. When false the route answers "Bootstrap is disabled". The code default is true so a fresh install works; the committed `docker-compose.yaml` defaults it to false — flip it to true for the one-time call, then back.

### 3. Hard rules

- The route is registered in `admin-api` only, behind the `auth` rate-limit tier
- The password is hashed with bcrypt cost 12 before the transaction opens
- The admin role and `Online` status come from migrated lookup data; a missing row is an internal error, not a silent insert
- Log the bootstrap at `warn` with the new user id and never log the password
- Never expose the bootstrap route through the api-gateway for customer traffic

## MFA / Two-Factor Authentication (TOTP)

### 1. Enrollment (already-authenticated user manages their own MFA)

`admin-api` exposes `POST /users/2fa/setup`, `/users/2fa/verify` and `/users/2fa/disable` for the signed-in user, each in the `sensitive` rate-limit tier. The TOTP secret is generated with `totp-rs`, encrypted with `CryptoUtil` (AES-256-GCM, stored as `iv:tag:ciphertext`) using `TWO_FACTOR_ENCRYPTION_KEY` (64 hex characters), and stored on the user. It is never returned after confirmation and never logged.

### 2. Post-login check (the part that is easy to forget)

When `twoFactorEnabled` is true, `login` must **not** return real tokens after the password check. It returns a short-lived `mfaToken` (the 5-minute challenge token) with `mfaRequired: true`. Real tokens are issued only by `POST /auth/verify-login-mfa` after the challenge token verifies and the TOTP code passes (with a limited number of attempts, tracked in Redis and locked for 5 minutes). Never treat password-correct as login-complete for an MFA-enabled user, and never accept a challenge token anywhere `authenticate` runs.

### UI side

See `rules/frontend.md`'s "MFA enrollment and login challenge" section for the enrollment page and the post-login challenge screen pattern.

## Critical rules

- Never use `unwrap`/`expect` on token operations — return `AppError::Unauthorized`
- Never put secrets, password hashes or TOTP secrets in a JWT, a response, or a log line
- Never share a secret between access and refresh tokens
- Never skip the blacklist check or the `minIat` check in `authenticate`
- Never issue real tokens before MFA completes for an MFA-enabled user
- Never add a second route that can create a `SUPER_ADMIN`
- Always rotate the refresh token on every refresh and blacklist the old `jti`
- Always add tests: login success and failure, lockout, refresh rotation and replay, logout revoking a second live token, MFA two-step, bootstrap once-only (see `tests/services/auth_service.rs` and `tests/integration` in `admin-api`)
