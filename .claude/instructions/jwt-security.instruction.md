---
applyTo: "apps/backend/*/src/**/*.rs,common/auth/src/**/*.rs,common/http/src/middleware/auth.rs"
---

# JWT Security — Token Revocation & Hijack Mitigation

> For generating the actual auth route/service/controller code (login, logout, refresh) that implements these rules, use the `jwt-security` subagent (`.claude/agents/jwt-security.md`) alongside `api-builder`. This file is the reference for the *why* and the mandatory rules; the agent is the *how*.

## Overview

JWTs are stateless by design. A client-side logout (clearing storage) does **not** invalidate a stolen token. This instruction defines the mandatory pattern for token lifecycle management across all services in this monorepo. The implementation lives in `common/auth` (`TokenService`) and `common/http` (`authenticate`).

## Token architecture

| Token | TTL | Secret | Server state |
|---|---|---|---|
| Access | 1 hour | `JWT_SECRET` | `jti` blacklist, per-user `minIat` |
| Refresh | 1 day, 30 days with `rememberMe` | `JWT_REFRESH_SECRET` | Redis allow-list `token:refresh:{userId}:{jti}` |
| MFA challenge | 5 minutes | `JWT_SECRET` | None; single purpose, not a session |

- Access tokens are short-lived to limit the blast radius of a stolen token. Clients keep them in memory, never in `localStorage` or `sessionStorage`.
- Refresh tokens are the revocable anchor: they exist only while their allow-list key exists, and they are rotated on every use.
- Both secrets must be at least 32 characters and must differ.
- Algorithm is pinned to HS256; `exp` is required; leeway is zero; the `type` claim must match the verifier.

## Required JWT payload shape

```rust
pub struct TokenPayload {
    pub id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<Permission>,
    pub scope: TokenScope,
    pub jti: String,
    #[serde(rename = "type")]
    pub token_type: TokenType,
    pub iat: i64,
    pub exp: i64,
}
```

`jti` is a fresh `Uuid::new_v4()` on every token. `scope` is `Customer` or `Admin`; each API's authenticator rejects tokens whose scope is not its own.

## Blacklist — Redis (required)

`token:blacklist:{jti}` is written with `RedisService::set_ex` and a TTL equal to the token's remaining useful life (1 hour for access tokens, 30 days for refresh tokens). `authenticate` rejects a token whose `jti` is present. A Redis error while checking is treated as "blacklisted" (fail closed).

## Logout

`TokenService::logout(user_id, access_token, refresh_token)`:

1. Delete every `token:refresh:{userId}:*` key.
2. Write `token:minIat:{userId}` with the cutoff `now + 1` and a 1 hour TTL.
3. Blacklist the access `jti` and the refresh `jti` that were presented.

The request is then authenticated against `iat < minIat`, strictly less-than. The `+ 1` cutoff makes every token minted in the logout second invalid, while a fresh login one second later is accepted. This is why logout invalidates every device and tab, not only the caller.

## Refresh token rotation

1. Verify signature and `type = refresh`.
2. Reject a blacklisted `jti`.
3. Require the allow-list key to exist; a valid-but-unlisted token is a replay and is rejected.
4. Blacklist the old `jti`, delete its allow-list key, issue a new pair, store the new allow-list key.

## Delivery

The API returns `accessToken` and `refreshToken` in the JSON body. Clients send the refresh token in the body of `POST /auth/refresh`. `admin-api` additionally sets it as the `rg_refresh` cookie (`HttpOnly`, `SameSite=Strict`, `Secure` in production, `Path=/api/v1/auth`, `Max-Age` equal to the refresh lifetime) and accepts it on `POST /auth/refresh` when the body has no `refreshToken`. `admin-web` keeps only the access token in memory, restores the session on load by calling `/auth/refresh` with the cookie, and refreshes once on a 401 before retrying. Any other cookie use keeps the same attributes.

## MFA

When a user has `twoFactorEnabled`, login returns `mfaRequired` with a challenge token instead of tokens. `POST /auth/verify-login-mfa` validates the challenge token and the TOTP code and only then issues real tokens. TOTP secrets are encrypted at rest with AES-256-GCM using `TWO_FACTOR_ENCRYPTION_KEY`.

## Rules for AI agents

- Every access token carries a unique `jti`
- Logout always goes through `TokenService::logout`; never delete a single key by hand
- Refresh always rotates; never reuse a refresh token
- `authenticate` always checks scope, blacklist and `minIat`, in that order, before loading the user
- Never log tokens, secrets or password hashes
- Never accept an MFA challenge token as a session
- Never return a token from any route that does not complete the full login (password and, when enabled, MFA)
