use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct LoginRecord {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub role_name: String,
    pub allow_email_communications: bool,
    pub two_factor_enabled: bool,
}
