use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_types::{RoleName, get_permissions_for_role};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::auth_config::AuthConfig;
use crate::claims::{MfaChallengePayload, TokenPair, TokenPayload, TokenType};

const BLACKLIST_PREFIX: &str = "token:blacklist:";
const REFRESH_PREFIX: &str = "token:refresh:";
const MIN_IAT_PREFIX: &str = "token:minIat:";
const ACCESS_TTL_SECONDS: i64 = 3600;
const MFA_CHALLENGE_TTL_SECONDS: i64 = 300;
const SHORT_SESSION_TTL_SECONDS: i64 = 24 * 3600;
const LONG_SESSION_TTL_SECONDS: i64 = 30 * 24 * 3600;
const REFRESH_BLACKLIST_TTL_SECONDS: u64 = 30 * 24 * 3600;
const ALGORITHM: Algorithm = Algorithm::HS256;

#[derive(Clone)]
pub struct TokenService {
    config: AuthConfig,
    redis: RedisService,
}

fn refresh_ttl(remember_me: bool) -> i64 {
    if remember_me {
        LONG_SESSION_TTL_SECONDS
    } else {
        SHORT_SESSION_TTL_SECONDS
    }
}

impl TokenService {
    pub fn new(config: AuthConfig, redis: RedisService) -> Self {
        Self { config, redis }
    }

    fn sign<T: Serialize>(claims: &T, secret: &str) -> Option<String> {
        encode(
            &Header::new(ALGORITHM),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .ok()
    }

    fn verify<T: DeserializeOwned>(token: &str, secret: &str) -> Option<T> {
        let mut validation = Validation::new(ALGORITHM);
        validation.leeway = 0;
        validation.required_spec_claims = ["exp".to_owned()].into_iter().collect();
        decode::<T>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .ok()
    }

    pub async fn generate_token(
        &self,
        user_id: &str,
        username: &str,
        role_name: &str,
        remember_me: bool,
    ) -> Option<TokenPair> {
        let permissions = RoleName::from_name(role_name)
            .map(get_permissions_for_role)
            .unwrap_or_default();
        let now = Utc::now().timestamp();
        let refresh_token_id = Uuid::new_v4().to_string();
        let build = |jti: String, token_type: TokenType, lifetime: i64| TokenPayload {
            id: user_id.to_owned(),
            username: username.to_owned(),
            role: role_name.to_owned(),
            permissions: permissions.clone(),
            scope: self.config.scope,
            jti,
            token_type,
            iat: now,
            exp: now + lifetime,
        };
        let access = build(
            Uuid::new_v4().to_string(),
            TokenType::Access,
            ACCESS_TTL_SECONDS,
        );
        let refresh = build(
            refresh_token_id.clone(),
            TokenType::Refresh,
            refresh_ttl(remember_me),
        );
        let access_token = Self::sign(&access, &self.config.access_secret)?;
        let refresh_token = Self::sign(&refresh, &self.config.refresh_secret)?;
        self.store_refresh_token(user_id, &refresh_token_id, remember_me)
            .await;
        Some(TokenPair {
            access_token,
            refresh_token,
            refresh_token_id,
        })
    }

    pub fn generate_mfa_challenge_token(&self, user_id: &str) -> Option<String> {
        let now = Utc::now().timestamp();
        Self::sign(
            &MfaChallengePayload {
                id: user_id.to_owned(),
                token_type: TokenType::MfaChallenge,
                iat: now,
                exp: now + MFA_CHALLENGE_TTL_SECONDS,
            },
            &self.config.access_secret,
        )
    }

    pub fn verify_mfa_challenge_token(&self, token: &str) -> Option<MfaChallengePayload> {
        Self::verify::<MfaChallengePayload>(token, &self.config.access_secret)
            .filter(|payload| payload.token_type == TokenType::MfaChallenge)
    }

    pub fn verify_access_token(&self, token: &str) -> Option<TokenPayload> {
        Self::verify::<TokenPayload>(token, &self.config.access_secret)
            .filter(|payload| payload.token_type == TokenType::Access)
    }

    pub fn verify_refresh_token(&self, token: &str) -> Option<TokenPayload> {
        Self::verify::<TokenPayload>(token, &self.config.refresh_secret)
            .filter(|payload| payload.token_type == TokenType::Refresh)
    }

    pub async fn refresh_token(&self, token: &str, remember_me: bool) -> Option<TokenPair> {
        let payload = self.verify_refresh_token(token)?;
        if self.is_token_blacklisted(&payload.jti).await {
            tracing::warn!(user_id = %payload.id, "attempted use of blacklisted refresh token");
            return None;
        }
        if !self.is_refresh_token_valid(&payload.id, &payload.jti).await {
            tracing::warn!(user_id = %payload.id, "refresh token not found or invalidated");
            return None;
        }
        self.blacklist_token(
            &payload.jti,
            u64::try_from(refresh_ttl(remember_me)).unwrap_or(0),
        )
        .await;
        self.remove_refresh_token(&payload.id, &payload.jti).await;
        let pair = self
            .generate_token(&payload.id, &payload.username, &payload.role, remember_me)
            .await;
        if pair.is_some() {
            tracing::info!(user_id = %payload.id, "refresh token rotated successfully");
        }
        pair
    }

    pub async fn logout(
        &self,
        user_id: &str,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) {
        self.invalidate_all_user_refresh_tokens(user_id).await;
        self.invalidate_all_access_tokens(user_id).await;
        if let Some(payload) = access_token.and_then(|token| self.verify_access_token(token)) {
            self.blacklist_token(&payload.jti, ACCESS_TTL_SECONDS.unsigned_abs())
                .await;
        }
        if let Some(payload) = refresh_token.and_then(|token| self.verify_refresh_token(token)) {
            self.blacklist_token(&payload.jti, REFRESH_BLACKLIST_TTL_SECONDS)
                .await;
            self.remove_refresh_token(user_id, &payload.jti).await;
        }
        tracing::info!(user_id, "user logged out successfully");
    }

    pub async fn is_token_blacklisted(&self, token_id: &str) -> bool {
        if !self.redis.is_available() {
            return false;
        }
        match self
            .redis
            .exists(&format!("{BLACKLIST_PREFIX}{token_id}"))
            .await
        {
            Ok(exists) => exists,
            Err(error) => {
                tracing::error!(%error, "failed to check token blacklist");
                true
            }
        }
    }

    pub async fn invalidate_all_access_tokens(&self, user_id: &str) {
        if !self.redis.is_available() {
            return;
        }
        let cutoff = (Utc::now().timestamp() + 1).to_string();
        if let Err(error) = self
            .redis
            .set_ex(
                &format!("{MIN_IAT_PREFIX}{user_id}"),
                ACCESS_TTL_SECONDS.unsigned_abs(),
                &cutoff,
            )
            .await
        {
            tracing::error!(%error, "failed to invalidate access tokens");
        }
    }

    pub async fn is_session_invalidated(&self, user_id: &str, issued_at: i64) -> bool {
        if !self.redis.is_available() {
            return false;
        }
        match self.redis.get(&format!("{MIN_IAT_PREFIX}{user_id}")).await {
            Ok(Some(raw)) => raw.parse::<i64>().is_ok_and(|min_iat| issued_at < min_iat),
            Ok(None) => false,
            Err(error) => {
                tracing::error!(%error, "failed to check session invalidation");
                true
            }
        }
    }

    async fn blacklist_token(&self, token_id: &str, ttl_seconds: u64) {
        if !self.redis.is_available() {
            return;
        }
        if let Err(error) = self
            .redis
            .set_ex(&format!("{BLACKLIST_PREFIX}{token_id}"), ttl_seconds, "1")
            .await
        {
            tracing::error!(%error, "failed to blacklist token");
        }
    }

    async fn store_refresh_token(&self, user_id: &str, token_id: &str, remember_me: bool) {
        if !self.redis.is_available() {
            return;
        }
        let value = serde_json::json!({
            "createdAt": Utc::now().timestamp_millis(),
            "rememberMe": remember_me,
        })
        .to_string();
        if let Err(error) = self
            .redis
            .set_ex(
                &format!("{REFRESH_PREFIX}{user_id}:{token_id}"),
                refresh_ttl(remember_me).unsigned_abs(),
                &value,
            )
            .await
        {
            tracing::error!(%error, "failed to store refresh token");
        }
    }

    async fn is_refresh_token_valid(&self, user_id: &str, token_id: &str) -> bool {
        if !self.redis.is_available() {
            return false;
        }
        self.redis
            .exists(&format!("{REFRESH_PREFIX}{user_id}:{token_id}"))
            .await
            .unwrap_or_else(|error| {
                tracing::error!(%error, "failed to validate refresh token");
                false
            })
    }

    async fn remove_refresh_token(&self, user_id: &str, token_id: &str) {
        if !self.redis.is_available() {
            return;
        }
        if let Err(error) = self
            .redis
            .del(&format!("{REFRESH_PREFIX}{user_id}:{token_id}"))
            .await
        {
            tracing::error!(%error, "failed to remove refresh token");
        }
    }

    async fn invalidate_all_user_refresh_tokens(&self, user_id: &str) {
        if !self.redis.is_available() {
            return;
        }
        let pattern = format!("{REFRESH_PREFIX}{user_id}:*");
        match self.redis.keys_matching(&pattern).await {
            Ok(keys) => {
                if let Err(error) = self.redis.del_many(&keys).await {
                    tracing::error!(%error, "failed to invalidate user refresh tokens");
                }
            }
            Err(error) => tracing::error!(%error, "failed to list user refresh tokens"),
        }
    }
}
