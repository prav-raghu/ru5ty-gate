use sqlx::FromRow;
use sqlx::postgres::PgRow;

pub trait Entity: for<'row> FromRow<'row, PgRow> + Send + Unpin {
    const TABLE: &'static str;
    const ORDER_BY: &'static str = "created_at DESC, id";
}
