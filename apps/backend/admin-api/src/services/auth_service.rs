use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use ru5ty_gate_auth::TokenService;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_database::{PgPool, sqlx};
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_http::AppError;
use ru5ty_gate_types::{ApiResponse, RoleName};
use ru5ty_gate_utilities::PasswordUtil;
use uuid::Uuid;

use crate::dtos::{CurrentUserDto, CurrentUserRecord, LoginData, LoginRecord, RefreshData};
use crate::schemas::{LoginRequest, ResetPasswordRequest, VerifyLoginMfaRequest};
use crate::services::UserService;

const LOGIN_MAX_ATTEMPTS: i64 = 5;
const LOGIN_LOCKOUT_TTL_SECONDS: i64 = 900;
const MFA_MAX_ATTEMPTS: i64 = 5;
const MFA_LOCKOUT_TTL_SECONDS: i64 = 300;
const GENERIC_CREDENTIAL_ERROR: &str = "Invalid username or password";
const FORGOT_NEUTRAL: &str = "If that email exists, a reset link has been sent";

#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    tokens: TokenService,
    email: Arc<dyn EmailSender>,
    redis: RedisService,
    users: UserService,
    admin_web_url: String,
    reset_expiration_minutes: i64,
}

fn failed_login(message: &str) -> ApiResponse<LoginData> {
    ApiResponse {
        is_successful: false,
        data: Some(LoginData::default()),
        message: Some(message.to_owned()),
        errors: None,
        date_time_stamp: None,
    }
}

fn message_only(success: bool, message: &str) -> ApiResponse<()> {
    ApiResponse {
        is_successful: success,
        data: None,
        message: Some(message.to_owned()),
        errors: None,
        date_time_stamp: None,
    }
}

fn is_admin_tier(role_name: &str) -> bool {
    RoleName::from_name(role_name).is_some_and(RoleName::is_admin_tier)
}

impl AuthService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: PgPool,
        tokens: TokenService,
        email: Arc<dyn EmailSender>,
        redis: RedisService,
        users: UserService,
        admin_web_url: String,
        reset_expiration_minutes: i64,
    ) -> Self {
        Self {
            pool,
            tokens,
            email,
            redis,
            users,
            admin_web_url,
            reset_expiration_minutes,
        }
    }

    async fn attempts(&self, key: &str) -> i64 {
        match self.redis.get(key).await {
            Ok(Some(raw)) => raw.parse().unwrap_or(0),
            _ => 0,
        }
    }

    async fn record_attempt(&self, key: &str, ttl_seconds: i64) {
        if let Ok(1) = self.redis.incr(key).await {
            let _ = self.redis.expire(key, ttl_seconds).await;
        }
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

    async fn send_login_notification(&self, record: &LoginRecord, ip: &str) {
        if !record.allow_email_communications {
            return;
        }
        let variables = BTreeMap::from([
            ("username".to_owned(), record.username.clone()),
            ("dateLoggedIn".to_owned(), Utc::now().to_rfc2822()),
            ("ipAddress".to_owned(), ip.to_owned()),
        ]);
        let _ = self
            .email
            .send_mail(
                &record.email,
                "Admin Login Notification",
                "admin-login-notification",
                &variables,
            )
            .await;
    }

    async fn issue_session(
        &self,
        record: &LoginRecord,
        remember_me: bool,
        ip: &str,
    ) -> Result<ApiResponse<LoginData>, AppError> {
        let pair = self
            .tokens
            .generate_token(
                &record.id.to_string(),
                &record.username,
                &record.role_name,
                remember_me,
            )
            .await
            .ok_or_else(|| AppError::Internal("token generation failed".to_owned()))?;
        self.set_status(record.id, "Online").await?;
        self.send_login_notification(record, ip).await;
        Ok(ApiResponse::success_with_message(
            LoginData {
                auth_token: pair.access_token,
                refresh_token: pair.refresh_token,
                username: record.username.clone(),
                mfa_required: None,
                mfa_token: None,
            },
            "Login successful",
        ))
    }

    async fn find_login_record(
        &self,
        column_is_id: bool,
        key: &str,
    ) -> Result<Option<LoginRecord>, AppError> {
        let query = if column_is_id {
            "SELECT u.id, u.username, u.email, u.password, r.name AS role_name, \
             u.allow_email_communications, u.two_factor_enabled \
             FROM users u JOIN roles r ON r.id = u.role_id WHERE u.id::text = $1 AND u.is_active = TRUE"
        } else {
            "SELECT u.id, u.username, u.email, u.password, r.name AS role_name, \
             u.allow_email_communications, u.two_factor_enabled \
             FROM users u JOIN roles r ON r.id = u.role_id WHERE u.email = $1 AND u.is_active = TRUE"
        };
        sqlx::query_as::<_, LoginRecord>(query)
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)
    }

    pub async fn login(
        &self,
        request: &LoginRequest,
        ip: &str,
    ) -> Result<ApiResponse<LoginData>, AppError> {
        let lock_key = format!("login:fail:{}", request.email);
        if self.attempts(&lock_key).await >= LOGIN_MAX_ATTEMPTS {
            return Ok(failed_login(
                "Account temporarily locked due to too many failed attempts. Try again later.",
            ));
        }
        let record = self.find_login_record(false, &request.email).await?;
        let password_valid = PasswordUtil::verify_or_dummy(
            &request.password,
            record.as_ref().map(|found| found.password.as_str()),
        )
        .await;
        let Some(record) = record.filter(|found| password_valid && is_admin_tier(&found.role_name))
        else {
            self.record_attempt(&lock_key, LOGIN_LOCKOUT_TTL_SECONDS)
                .await;
            return Ok(failed_login(GENERIC_CREDENTIAL_ERROR));
        };
        let _ = self.redis.del(&lock_key).await;

        if record.two_factor_enabled {
            let mfa_token = self
                .tokens
                .generate_mfa_challenge_token(&record.id.to_string())
                .ok_or_else(|| AppError::Internal("mfa token generation failed".to_owned()))?;
            return Ok(ApiResponse::success_with_message(
                LoginData {
                    mfa_required: Some(true),
                    mfa_token: Some(mfa_token),
                    ..LoginData::default()
                },
                "Verification code required",
            ));
        }
        self.issue_session(&record, request.remember_me, ip).await
    }

    pub async fn verify_login_mfa(
        &self,
        request: &VerifyLoginMfaRequest,
        ip: &str,
    ) -> Result<ApiResponse<LoginData>, AppError> {
        let Some(challenge) = self.tokens.verify_mfa_challenge_token(&request.mfa_token) else {
            return Ok(failed_login(
                "Verification session expired - please log in again",
            ));
        };
        let lock_key = format!("mfa:fail:{}", challenge.id);
        if self.attempts(&lock_key).await >= MFA_MAX_ATTEMPTS {
            return Ok(failed_login(
                "Too many failed verification attempts. Please log in again.",
            ));
        }
        let Ok(user_id) = Uuid::parse_str(&challenge.id) else {
            return Ok(failed_login("Invalid verification session"));
        };
        if !self.users.verify_totp_code(user_id, &request.code).await? {
            self.record_attempt(&lock_key, MFA_LOCKOUT_TTL_SECONDS)
                .await;
            return Ok(failed_login("Invalid verification code"));
        }
        let _ = self.redis.del(&lock_key).await;
        let record = self.find_login_record(true, &challenge.id).await?;
        let Some(record) = record.filter(|found| is_admin_tier(&found.role_name)) else {
            return Ok(failed_login("Invalid verification session"));
        };
        self.issue_session(&record, request.remember_me, ip).await
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
    ) -> Result<ApiResponse<()>, AppError> {
        sqlx::query(
            "UPDATE users SET last_seen = NOW(), \
             user_status_id = (SELECT id FROM user_statuses WHERE name = 'Offline') \
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        self.tokens
            .logout(&user_id.to_string(), access_token, None)
            .await;
        Ok(message_only(true, "Successfully Logged Out"))
    }

    pub async fn forgot_password(&self, email: &str) -> Result<ApiResponse<()>, AppError> {
        let record = self.find_login_record(false, email).await?;
        let Some(record) = record.filter(|found| is_admin_tier(&found.role_name)) else {
            return Ok(message_only(true, FORGOT_NEUTRAL));
        };
        let reset_hash = Uuid::new_v4().to_string();
        sqlx::query("UPDATE users SET auth_hash = $2, auth_hash_expiration = $3 WHERE id = $1")
            .bind(record.id)
            .bind(&reset_hash)
            .bind(Utc::now() + Duration::minutes(self.reset_expiration_minutes))
            .execute(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let variables = BTreeMap::from([
            ("username".to_owned(), record.username.clone()),
            (
                "resetLink".to_owned(),
                format!("{}/reset-password?token={reset_hash}", self.admin_web_url),
            ),
        ]);
        let sent = self
            .email
            .send_mail(
                email,
                "Admin Password Reset",
                "admin-forgot-password",
                &variables,
            )
            .await;
        if sent {
            Ok(message_only(true, "Password reset email sent successfully"))
        } else {
            Ok(message_only(false, "Error occurred"))
        }
    }

    pub async fn reset_password(
        &self,
        request: &ResetPasswordRequest,
    ) -> Result<ApiResponse<()>, AppError> {
        if request.confirm_password.as_deref() != Some(request.password.as_str()) {
            return Ok(message_only(
                false,
                "New password and confirm password do not match",
            ));
        }
        let found = sqlx::query_as::<_, (Uuid, String, String, Option<chrono::DateTime<Utc>>)>(
            "SELECT u.id, u.email, r.name, u.auth_hash_expiration FROM users u \
             JOIN roles r ON r.id = u.role_id WHERE u.auth_hash = $1",
        )
        .bind(request.token.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((user_id, email, role_name, expiration)) = found else {
            return Ok(message_only(false, "Invalid or expired token"));
        };
        if !is_admin_tier(&role_name) {
            return Ok(message_only(false, "Invalid or expired token"));
        }
        if expiration.is_none_or(|expires_at| expires_at < Utc::now()) {
            return Ok(message_only(false, "Token has expired"));
        }
        let hash = PasswordUtil::hash(&request.password)
            .await
            .ok_or_else(|| AppError::Internal("password hashing failed".to_owned()))?;
        sqlx::query(
            "UPDATE users SET password = $2, auth_hash = NULL, auth_hash_expiration = NULL, \
             modified_by = $1::text WHERE id = $1",
        )
        .bind(user_id)
        .bind(hash)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        self.tokens.logout(&user_id.to_string(), None, None).await;
        let username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let variables = BTreeMap::from([("username".to_owned(), username)]);
        let _ = self
            .email
            .send_mail(
                &email,
                "Admin Password Reset",
                "admin-password-reset",
                &variables,
            )
            .await;
        Ok(message_only(true, "Password reset successful"))
    }

    pub async fn get_current_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<CurrentUserDto>, AppError> {
        let record = sqlx::query_as::<_, CurrentUserRecord>(
            "SELECT u.id, u.username, u.email, u.avatar, u.allow_email_communications, \
             u.two_factor_enabled, u.created_at, u.last_seen, \
             r.id AS role_id, r.name AS role_name, s.id AS status_id, s.name AS status_name \
             FROM users u \
             JOIN roles r ON r.id = u.role_id \
             JOIN user_statuses s ON s.id = u.user_status_id \
             WHERE u.id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(record.map(CurrentUserDto::from))
    }
}
