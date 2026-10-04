use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct SessionListFilter {
    pub venue_id: Uuid,
    pub open_only: bool,
    pub limit: i64,
    pub offset: i64,
}
