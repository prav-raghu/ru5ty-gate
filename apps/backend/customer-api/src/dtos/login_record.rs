use ru5ty_gate_database::sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct LoginRecord {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub role_name: String,
    pub status_name: String,
}
