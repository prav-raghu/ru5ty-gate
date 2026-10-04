use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use ru5ty_gate_database::{PgPool, sqlx};
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_http::{AppError, AuthUser};
use ru5ty_gate_types::{ApiResponse, Permission, RoleName, is_disposable_email};
use ru5ty_gate_utilities::{CryptoUtil, PasswordUtil};
use totp_rs::{Builder, Secret, Totp};
use uuid::Uuid;

use crate::dtos::{
    AuthorizedAdmin, AvailabilityByEmail, AvailabilityByUsername, NamedItem, ProfileData,
    Setup2FaData, UserDetailsDto, UserDetailsRecord,
};
use crate::schemas::{OnboardingRequest, UpdateProfileRequest};

const ONBOARDING_VALIDITY_HOURS: i64 = 24;
const RESEND_VALIDITY_HOURS: i64 = 2;
const TOTP_SKEW: u16 = 1;
const TOTP_STEP_SECONDS: u64 = 30;
const TOTP_ISSUER: &str = "Admin";

#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
    email: Arc<dyn EmailSender>,
    crypto: Arc<CryptoUtil>,
    admin_web_url: String,
}

fn build_totp(secret_base32: &str, account: &str) -> Result<Totp, AppError> {
    let secret = Secret::try_from_base32(secret_base32).map_err(AppError::internal)?;
    Builder::new()
        .with_secret(secret)
        .with_skew(TOTP_SKEW)
        .with_step_duration(TOTP_STEP_SECONDS)
        .with_account_name(account)
        .with_issuer(Some(TOTP_ISSUER))
        .build()
        .map_err(AppError::internal)
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

impl UserService {
    pub fn new(
        pool: PgPool,
        email: Arc<dyn EmailSender>,
        crypto: Arc<CryptoUtil>,
        admin_web_url: String,
    ) -> Self {
        Self {
            pool,
            email,
            crypto,
            admin_web_url,
        }
    }

    pub async fn get_authorized_admin(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AuthorizedAdmin>, AppError> {
        let admin_roles: Vec<&str> = ru5ty_gate_types::ADMIN_TIER_ROLES
            .iter()
            .map(|role| role.as_str())
            .collect();
        sqlx::query_as::<_, AuthorizedAdmin>(
            "SELECT u.id, u.username, u.email, r.name AS role_name \
             FROM users u \
             JOIN roles r ON r.id = u.role_id \
             JOIN user_statuses s ON s.id = u.user_status_id \
             WHERE u.id = $1 AND u.is_active = TRUE AND s.name = 'Online' AND r.name = ANY($2)",
        )
        .bind(user_id)
        .bind(&admin_roles)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)
    }

    pub async fn get_user_roles(&self) -> Result<ApiResponse<Vec<NamedItem>>, AppError> {
        let roles = sqlx::query_as::<_, (Uuid, String)>("SELECT id, name FROM roles ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::internal)?;
        Ok(ApiResponse::success_with_message(
            roles
                .into_iter()
                .map(|(id, name)| NamedItem { id, name })
                .collect(),
            "Roles retrieved successfully",
        ))
    }

    pub async fn get_user_statuses(&self) -> Result<ApiResponse<Vec<NamedItem>>, AppError> {
        let statuses =
            sqlx::query_as::<_, (Uuid, String)>("SELECT id, name FROM user_statuses ORDER BY name")
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::internal)?;
        Ok(ApiResponse::success_with_message(
            statuses
                .into_iter()
                .map(|(id, name)| NamedItem { id, name })
                .collect(),
            "Statuses retrieved successfully",
        ))
    }

    fn verification_link(&self, auth_hash: &str) -> String {
        format!("{}/verify-email?auth_hash={auth_hash}", self.admin_web_url)
    }

    async fn send_onboarding_email(&self, email: &str, username: &str, auth_hash: &str) -> bool {
        let variables = BTreeMap::from([
            ("username".to_owned(), username.to_owned()),
            ("dateLoggedIn".to_owned(), Utc::now().to_rfc2822()),
            (
                "verificationLink".to_owned(),
                self.verification_link(auth_hash),
            ),
        ]);
        self.email
            .send_mail(
                email,
                "Admin Onboarding",
                "admin-onboarding-notification",
                &variables,
            )
            .await
    }

    pub async fn onboard_user(
        &self,
        request: &OnboardingRequest,
        actor: &AuthUser,
    ) -> Result<ApiResponse<()>, AppError> {
        let existing = sqlx::query_as::<_, (String, String)>(
            "SELECT username, email FROM users WHERE username = $1 OR email = $2 LIMIT 1",
        )
        .bind(&request.username)
        .bind(&request.email)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        if let Some((username, _)) = existing {
            let message = if username == request.username {
                "Username already exists"
            } else {
                "Email already exists"
            };
            return Ok(message_only(false, message));
        }
        if is_disposable_email(&request.email) {
            return Ok(message_only(false, "Email address is not allowed"));
        }

        let role_name: Option<String> = sqlx::query_scalar("SELECT name FROM roles WHERE id = $1")
            .bind(request.role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let Some(role_name) = role_name else {
            return Ok(message_only(false, "User role not found"));
        };
        if RoleName::from_name(&role_name) == Some(RoleName::SuperAdmin)
            && !actor.permissions.contains(&Permission::RoleAssign)
        {
            return Err(AppError::Forbidden(
                "Forbidden: insufficient permissions".to_owned(),
            ));
        }
        let status_exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM user_statuses WHERE id = $1)")
                .bind(request.user_status_id)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::internal)?;
        if !status_exists {
            return Ok(message_only(false, "User status not found"));
        }

        let password_hash = PasswordUtil::hash(&request.password)
            .await
            .ok_or_else(|| AppError::Internal("password hashing failed".to_owned()))?;
        let auth_hash = Uuid::new_v4().to_string();
        let inserted = sqlx::query(
            "INSERT INTO users (username, email, password, role_id, gender, age, \
             accept_terms_and_conditions, allow_email_communications, ip_address, \
             user_status_id, avatar, auth_hash, auth_hash_expiration, created_by, modified_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)",
        )
        .bind(&request.username)
        .bind(&request.email)
        .bind(password_hash)
        .bind(request.role_id)
        .bind(request.gender.as_deref())
        .bind(request.age)
        .bind(request.accept_terms_and_conditions.unwrap_or(false))
        .bind(request.allow_email_communications)
        .bind(&request.ip_address)
        .bind(request.user_status_id)
        .bind(request.avatar.as_deref())
        .bind(&auth_hash)
        .bind(Utc::now() + Duration::hours(ONBOARDING_VALIDITY_HOURS))
        .bind(actor.id.to_string())
        .execute(&self.pool)
        .await;
        if let Err(error) = inserted {
            if error
                .as_database_error()
                .is_some_and(|database| database.is_unique_violation())
            {
                return Err(AppError::Conflict(
                    "Username or email already exists".to_owned(),
                ));
            }
            return Err(AppError::internal(error));
        }
        let _ = self
            .send_onboarding_email(&request.email, &request.username, &auth_hash)
            .await;
        Ok(message_only(true, "User onboarded successfully"))
    }

    pub async fn resend_verification_email(
        &self,
        email: &str,
    ) -> Result<ApiResponse<()>, AppError> {
        let found = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT u.id, u.username, s.name FROM users u \
             JOIN user_statuses s ON s.id = u.user_status_id WHERE u.email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((user_id, username, status)) = found else {
            return Ok(message_only(false, "User not found"));
        };
        if status != "Pending Verification" {
            return Ok(message_only(
                false,
                "You cannot resend verification email for this user",
            ));
        }
        let auth_hash = Uuid::new_v4().to_string();
        sqlx::query("UPDATE users SET auth_hash = $2, auth_hash_expiration = $3 WHERE id = $1")
            .bind(user_id)
            .bind(&auth_hash)
            .bind(Utc::now() + Duration::hours(RESEND_VALIDITY_HOURS))
            .execute(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let _ = self
            .send_onboarding_email(email, &username, &auth_hash)
            .await;
        Ok(message_only(true, "Verification email resent successfully"))
    }

    pub async fn is_email_available(&self, email: &str) -> Result<AvailabilityByEmail, AppError> {
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
                .bind(email)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::internal)?;
        Ok(AvailabilityByEmail {
            email: email.to_owned(),
            available: !taken,
        })
    }

    pub async fn is_username_available(
        &self,
        username: &str,
    ) -> Result<AvailabilityByUsername, AppError> {
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
                .bind(username)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::internal)?;
        Ok(AvailabilityByUsername {
            username: username.to_owned(),
            available: !taken,
        })
    }

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        request: &UpdateProfileRequest,
    ) -> Result<ApiResponse<ProfileData>, AppError> {
        let existing = sqlx::query_as::<_, (String, String)>(
            "SELECT username, email FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((current_username, current_email)) = existing else {
            return Ok(ApiResponse::failure("User not found"));
        };
        if request.username != current_username {
            let taken: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
                    .bind(&request.username)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(AppError::internal)?;
            if taken {
                return Ok(ApiResponse::failure("Username already taken"));
            }
        }
        if request.email != current_email {
            if is_disposable_email(&request.email) {
                return Ok(ApiResponse::failure("Email address is not allowed"));
            }
            let taken: bool =
                sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
                    .bind(&request.email)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(AppError::internal)?;
            if taken {
                return Ok(ApiResponse::failure("Email already taken"));
            }
        }
        let updated = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
            "UPDATE users SET username = $2, email = $3, avatar = COALESCE($4, avatar), \
             modified_by = $1::text WHERE id = $1 RETURNING id, username, email, avatar",
        )
        .bind(user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(request.avatar.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(ApiResponse::success_with_message(
            ProfileData {
                id: updated.0,
                username: updated.1,
                email: updated.2,
                avatar: updated.3,
            },
            "Profile updated successfully",
        ))
    }

    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<ApiResponse<()>, AppError> {
        let hash: Option<String> = sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let Some(hash) = hash else {
            return Ok(message_only(false, "User not found"));
        };
        if !PasswordUtil::verify(current_password, &hash).await {
            return Ok(message_only(false, "Current password is incorrect"));
        }
        let new_hash = PasswordUtil::hash(new_password)
            .await
            .ok_or_else(|| AppError::Internal("password hashing failed".to_owned()))?;
        sqlx::query("UPDATE users SET password = $2, modified_by = $1::text WHERE id = $1")
            .bind(user_id)
            .bind(new_hash)
            .execute(&self.pool)
            .await
            .map_err(AppError::internal)?;
        Ok(message_only(true, "Password changed successfully"))
    }

    pub async fn setup_2fa(&self, user_id: Uuid) -> Result<ApiResponse<Setup2FaData>, AppError> {
        let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::internal)?;
        let Some(email) = email else {
            return Ok(ApiResponse::failure("User not found"));
        };
        let secret = Secret::generate().to_base32();
        let totp = build_totp(&secret, &email)?;
        let qr_code = totp.to_qr_base64().map_err(AppError::internal)?;
        let encrypted = self.crypto.encrypt(&secret).map_err(AppError::internal)?;
        sqlx::query("UPDATE users SET two_factor_secret = $2 WHERE id = $1")
            .bind(user_id)
            .bind(encrypted)
            .execute(&self.pool)
            .await
            .map_err(AppError::internal)?;
        Ok(ApiResponse::success_with_message(
            Setup2FaData {
                secret,
                qr_code: format!("data:image/png;base64,{qr_code}"),
            },
            "2FA setup initiated",
        ))
    }

    pub async fn verify_totp_code(&self, user_id: Uuid, code: &str) -> Result<bool, AppError> {
        let row = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT email, two_factor_secret FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        let Some((email, Some(encrypted))) = row else {
            return Ok(false);
        };
        let secret = self
            .crypto
            .decrypt(&encrypted)
            .map_err(AppError::internal)?;
        let totp = build_totp(&secret, &email)?;
        Ok(totp.check_current(code).is_some())
    }

    pub async fn verify_2fa(
        &self,
        user_id: Uuid,
        token: &str,
    ) -> Result<ApiResponse<()>, AppError> {
        let has_secret: bool = sqlx::query_scalar(
            "SELECT COALESCE((SELECT two_factor_secret IS NOT NULL FROM users WHERE id = $1), FALSE)",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::internal)?;
        if !has_secret {
            return Ok(message_only(false, "User not found or 2FA not set up"));
        }
        if !self.verify_totp_code(user_id, token).await? {
            return Ok(message_only(false, "Invalid verification code"));
        }
        sqlx::query(
            "UPDATE users SET two_factor_enabled = TRUE, modified_by = $1::text WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(message_only(true, "2FA enabled successfully"))
    }

    pub async fn disable_2fa(
        &self,
        user_id: Uuid,
        token: &str,
    ) -> Result<ApiResponse<()>, AppError> {
        let enabled: bool = sqlx::query_scalar(
            "SELECT COALESCE((SELECT two_factor_enabled AND two_factor_secret IS NOT NULL \
             FROM users WHERE id = $1), FALSE)",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::internal)?;
        if !enabled {
            return Ok(message_only(false, "User not found or 2FA not enabled"));
        }
        if !self.verify_totp_code(user_id, token).await? {
            return Ok(message_only(false, "Invalid verification code"));
        }
        sqlx::query(
            "UPDATE users SET two_factor_enabled = FALSE, two_factor_secret = NULL, \
             modified_by = $1::text WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(message_only(true, "2FA disabled successfully"))
    }

    pub async fn get_user_details(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserDetailsDto>, AppError> {
        let record = sqlx::query_as::<_, UserDetailsRecord>(
            "SELECT u.id, u.username, u.email, u.avatar, u.gender, u.age, \
             u.accept_terms_and_conditions, u.allow_email_communications, u.ip_address, \
             u.last_seen, u.is_active, u.two_factor_enabled, u.user_status_id, u.role_id, \
             u.created_at, u.updated_at, u.created_by, u.modified_by, \
             s.name AS status_name, s.is_active AS status_is_active, \
             s.created_at AS status_created_at, s.updated_at AS status_updated_at, \
             s.created_by AS status_created_by, s.modified_by AS status_modified_by, \
             r.name AS role_name, r.is_active AS role_is_active, \
             r.created_at AS role_created_at, r.updated_at AS role_updated_at, \
             r.created_by AS role_created_by, r.modified_by AS role_modified_by \
             FROM users u \
             JOIN user_statuses s ON s.id = u.user_status_id \
             JOIN roles r ON r.id = u.role_id \
             WHERE u.id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::internal)?;
        Ok(record.map(UserDetailsDto::from))
    }
}
