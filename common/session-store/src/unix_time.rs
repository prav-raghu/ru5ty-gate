const BUILD_UNIX: &str = env!("RU5TY_GATE_BUILD_UNIX");

pub fn unix_now() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn build_unix() -> i64 {
    BUILD_UNIX.parse().unwrap_or(0)
}

pub fn clock_is_trusted(now: i64) -> bool {
    now >= build_unix()
}
