use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct UserListFilter<'a> {
    pub exclude_user_id: Option<Uuid>,
    pub gender: Option<&'a str>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    pub limit: i64,
    pub offset: i64,
}
