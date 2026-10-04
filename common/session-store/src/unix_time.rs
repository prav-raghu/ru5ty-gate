pub fn unix_now() -> i64 {
    chrono::Utc::now().timestamp()
}
