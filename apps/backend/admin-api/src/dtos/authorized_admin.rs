use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct AuthorizedAdmin {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role_name: String,
}
