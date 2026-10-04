#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    pub limit: i64,
    pub offset: i64,
}

impl PageRequest {
    pub const DEFAULT_LIMIT: i64 = 20;
    pub const MAX_LIMIT: i64 = 100;

    pub fn new(limit: Option<i64>, offset: Option<i64>) -> Self {
        Self {
            limit: limit
                .unwrap_or(Self::DEFAULT_LIMIT)
                .clamp(1, Self::MAX_LIMIT),
            offset: offset.unwrap_or(0).max(0),
        }
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self::new(None, None)
    }
}
