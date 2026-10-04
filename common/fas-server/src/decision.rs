pub enum Decision {
    Grant {
        duration_secs: u64,
        redirect_url: Option<String>,
    },
    Deny {
        reason: String,
    },
}
