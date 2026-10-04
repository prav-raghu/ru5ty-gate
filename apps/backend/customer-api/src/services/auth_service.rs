use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, LazyLock};

use chrono::{Duration, Utc};
use ru5ty_gate_auth::TokenService;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_database::{PgPool, sqlx};
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::{ApiResponse, DISPOSABLE_EMAIL_DOMAINS, RoleName, is_disposable_email};
use ru5ty_gate_utilities::PasswordUtil;
use uuid::Uuid;

use crate::dtos::{LoginData, LoginRecord, RefreshData, RegisterData};
use crate::schemas::{LoginRequest, RegisterRequest};

const LOGIN_MAX_ATTEMPTS: i64 = 5;
const LOGIN_LOCKOUT_TTL_SECONDS: i64 = 900;
const VERIFICATION_VALIDITY_HOURS: i64 = 2;
const NEUTRAL_RESEND_MESSAGE: &str =
    "If that email is registered and unverified, a new link has been sent";
const PROHIBITED_DOMAINS_JSON: &str = include_str!("../data/prohibited-email-domains.json");

static PROHIBITED_DOMAINS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let configured: Vec<String> =
        serde_json::from_str::<serde_json::Value>(PROHIBITED_DOMAINS_JSON)
            .ok()
            .and_then(|value| value.get("data").cloned())
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
    configured
        .into_iter()
        .map(|domain| domain.to_ascii_lowercase())
        .chain(
            DISPOSABLE_EMAIL_DOMAINS
                .iter()
                .map(|domain| (*domain).to_owned()),
        )
        .collect()
});

pub fn is_username_valid(username: &str) -> bool {
    !username.trim().is_empty()
        && username.chars().all(|character| {
            character.is_ascii_alphabetic() || character == '_' || character == ' '
        })
}

pub fn is_email_domain_allowed(email: &str) -> bool {
    let Some(domain) = email
        .rsplit_once('@')
        .map(|(_, domain)| domain.to_ascii_lowercase())
    else {
        return false;
    };
    !domain.is_empty() && !PROHIBITED_DOMAINS.contains(&domain) && !is_disposable_email(email)
}

#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    tokens: TokenService,
    email: Arc<dyn EmailSender>,
    redis: RedisService,
    customer_web_url: String,
}

impl AuthService {
    pub fn new(
        pool: PgPool,
        tokens: TokenService,
        email: Arc<dyn EmailSender>,
        redis: RedisService,
        customer_web_url: String,
    ) -> Self {
        Self {
            pool,
            tokens,
            email,
            redis,
            customer_web_url,
        }
    }

    async fn role_id(&self, name: &str) -> Result<Uuid, AppError> {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM roles WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)?
            .ok_or_else(|| AppError::Internal(format!("role '{name}' is missing")))
    }

    async fn status_id(&self, name: &str) -> Result<Uuid, AppError> {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_statuses WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)?
            .ok_or_else(|| AppError::Internal(format!("user status '{name}' is missing")))
    }

    async fn is_username_taken(&self, username: &str) -> Result<bool, AppError> {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::internal)
    }

    async fn is_email_taken(&self, email: &str) -> Result<bool, AppError> {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
            .bind(email)
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::internal)
    }

    fn verification_link(&self, email: &str, token: &str) -> String {
        format!(
            "{}/verify-email?email={}&token={token}",
            self.customer_web_url,
            urlencoding::encode(email)
        )
    }

    async fn send_verification_email(&self, email: &str, username: &str, link: &str) -> bool {
        let variables = BTreeMap::from([
            ("email".to_owned(), email.to_owned()),
            ("username".to_owned(), username.to_owned()),
            ("verificationLink".to_owned(), link.to_owned()),
        ]);
        self.email
            .send_mail(email, "Verify your email", "verify-email", &variables)
            .await
    }

    pub async fn register(
        &self,
        request: &RegisterRequest,
        ip: &str,
    ) -> Result<ApiResponse<RegisterData>, AppError> {
        if !request.accept_terms_and_conditions {
            return Ok(ApiResponse::failure(
                "You must accept the terms and conditions to register",
            ));
        }
        if !is_username_valid(&request.username) {
            return Ok(ApiResponse::failure("Username contains invalid characters"));
        }
        if self.is_username_taken(&request.username).await? {
            return Ok(ApiResponse::failure("Username is already taken"));
        }
        if !is_email_domain_allowed(&request.email) {
            return Ok(ApiResponse::failure("Email address is not allowed"));
        }
        if self.is_email_taken(&request.email).await? {
            return Ok(ApiResponse::failure("Email is already registered"));
        }

        let password_hash = PasswordUtil::hash(&request.password)
            .await
            .ok_or_else(|| AppError::Internal("password hashing failed".to_owned()))?;
        let role_id = self.role_id(RoleName::ChatUser.as_str()).await?;
        let status_id = self.status_id("Pending Verification").await?;
        let auth_hash = Uuid::new_v4().to_string();
        let expiry = Utc::now() + Duration::hours(VERIFICATION_VALIDITY_HOURS);

        let inserted = sqlx::query(
            "INSERT INTO users (username, password, email, age, gender, \
             accept_terms_and_conditions, allow_email_communications, ip_address, role_id, \
             user_status_id, auth_hash, auth_hash_expiration) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&request.username)
        .bind(&password_hash)
        .bind(&request.email)
        .bind(request.age)
        .bind(&request.gender_id)
        .bind(request.accept_terms_and_conditions)
        .bind(request.allow_email_communications)
        .bind(ip)
        .bind(role_id)
        .bind(status_id)
        .bind(&auth_hash)
        .bind(expiry)
        .execute(&self.pool)
        .await;
        if let Err(error) = inserted {
            if error
                .as_database_error()
                .is_some_and(|database| database.is_unique_violation())
            {
                return Err(AppError::Conflict(
                    "Username or email is already registered".to_owned(),
                ));
            }
            return Err(AppError::internal(error));
        }

        let link = self.verification_link(&request.email, &auth_hash);
        if !self
            .send_verification_email(&request.email, &request.username, &link)
            .await
        {
            return Ok(ApiResponse::failure("Failed to send user verification email").stamped());
        }
        Ok(ApiResponse::success_with_message(
            RegisterData {
                email: request.email.clone(),
            },
            "Account registered successfully please check your email for verification instructions.",
        )
        .stamped())
    }

    async fn failed_attempts(&self, key: &str) -> i64 {
        match self.redis.get(key).await {
            Ok(Some(raw)) => raw.parse().unwrap_or(0),
            _ => 0,
        }
    }

    async fn record_failed_attempt(&self, key: &str) {
        if let Ok(1) = self.redis.incr(key).await {
            let _ = self.redis.expire(key, LOGIN_LOCKOUT_TTL_SECONDS).await;
        }
    }

    pub async fn login(&self, request: &LoginRequest) -> Result<ApiResponse<LoginData>, AppError> {
        let lock_key = format!("login:fail:{}", request.username);
        if self.failed_attempts(&lock_key).await >= LOGIN_MAX_ATTEMPTS {
            return Ok(ApiResponse::failure(
                "Account temporarily locked due to too many failed attempts. Try again later.",
            ));
        }

        let record = sqlx::query_as::<_, LoginRecord>(
            "SELECT u.id, u.username, u.email, u.password, r.name AS role_name, \
             s.name AS status_name \
             FROM users u \
             JOIN roles r ON r.id = u.role_id \
             JOIN user_statuses s ON s.id = u.user_status_id \
             WHERE u.username = $1 AND u.is_active = TRUE",
        )
        .bind(&request.username)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;

        let password_valid = PasswordUtil::verify_or_dummy(
            &request.password,
            record.as_ref().map(|found| found.password.as_str()),
        )
        .await;
        let customer_tier = record.as_ref().is_some_and(|found| {
            RoleName::from_name(&found.role_name).is_some_and(RoleName::is_customer_tier)
        });
        let Some(record) = record.filter(|_| password_valid && customer_tier) else {
            self.record_failed_attempt(&lock_key).await;
            return Ok(ApiResponse::failure("Invalid username or password"));
        };
        if record.status_name == "Pending Verification" {
            return Ok(ApiResponse::failure(
                "Please verify your email before logging in",
            ));
        }

        let _ = self.redis.del(&lock_key).await;
        self.set_status(record.id, "Online").await?;
        let tokens = self
            .tokens
            .generate_token(
                &record.id.to_string(),
                &record.username,
                &record.role_name,
                request.remember_me.unwrap_or(true),
            )
            .await
            .ok_or_else(|| AppError::Internal("token generation failed".to_owned()))?;
        Ok(ApiResponse::success_with_message(
            LoginData {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                user_name: record.username,
                email: record.email,
            },
            "Login successful",
        ))
    }

    pub async fn refresh_token(&self, token: &str, remember_me: bool) -> Option<RefreshData> {
        self.tokens
            .refresh_token(token, remember_me)
            .await
            .map(|pair| RefreshData {
                access_token: pair.access_token,
                refresh_token: pair.refresh_token,
            })
    }

    pub async fn logout(
        &self,
        user_id: Uuid,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) -> Result<(), AppError> {
        self.tokens
            .logout(&user_id.to_string(), access_token, refresh_token)
            .await;
        sqlx::query(
            "UPDATE users SET last_seen = NOW(), \
             user_status_id = (SELECT id FROM user_statuses WHERE name = 'Offline') \
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(())
    }

    async fn set_status(&self, user_id: Uuid, status: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET user_status_id = (SELECT id FROM user_statuses WHERE name = $2) \
             WHERE id = $1",
        )
        .bind(user_id)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(())
    }

    pub async fn verify_email(&self, token: &str) -> Result<ApiResponse<()>, AppError> {
        let found = sqlx::query_as::<_, (Uuid, String, Option<chrono::DateTime<Utc>>)>(
            "SELECT u.id, s.name, u.auth_hash_expiration FROM users u \
             JOIN user_statuses s ON s.id = u.user_status_id WHERE u.auth_hash = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((user_id, status, expiration)) = found else {
            return Ok(ApiResponse::failure("Invalid or expired token"));
        };
        if status == "Verified" {
            return Ok(ApiResponse::failure("Email is already verified"));
        }
        if expiration.is_none_or(|expires_at| expires_at <= Utc::now()) {
            return Ok(ApiResponse::failure("Invalid or expired token"));
        }
        sqlx::query(
            "UPDATE users SET auth_hash = NULL, auth_hash_expiration = NULL, \
             user_status_id = (SELECT id FROM user_statuses WHERE name = 'Verified') \
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(ApiResponse {
            is_successful: true,
            data: None,
            message: Some("Email verified successfully".to_owned()),
            errors: None,
            date_time_stamp: None,
        })
    }

    pub async fn resend_verification_email(
        &self,
        email: &str,
    ) -> Result<ApiResponse<()>, AppError> {
        let neutral = || ApiResponse::<()> {
            is_successful: true,
            data: None,
            message: Some(NEUTRAL_RESEND_MESSAGE.to_owned()),
            errors: None,
            date_time_stamp: None,
        };
        let found = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT u.id, u.username, s.name FROM users u \
             JOIN user_statuses s ON s.id = u.user_status_id WHERE u.email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((user_id, username, status)) = found else {
            return Ok(neutral());
        };
        if status == "Verified" {
            return Ok(neutral());
        }
        let auth_hash = Uuid::new_v4().to_string();
        sqlx::query("UPDATE users SET auth_hash = $2, auth_hash_expiration = $3 WHERE id = $1")
            .bind(user_id)
            .bind(&auth_hash)
            .bind(Utc::now() + Duration::hours(VERIFICATION_VALIDITY_HOURS))
            .execute(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let link = self.verification_link(email, &auth_hash);
        let _ = self.send_verification_email(email, &username, &link).await;
        Ok(neutral())
    }
}
