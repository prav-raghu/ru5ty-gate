#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub mac: String,
    pub token: String,
    pub venue: String,
    pub granted_at: i64,
    pub expires_at: i64,
    pub redirect_url: Option<String>,
}

impl Session {
    pub fn is_expired(&self, now: i64) -> bool {
        now >= self.expires_at
    }
}
