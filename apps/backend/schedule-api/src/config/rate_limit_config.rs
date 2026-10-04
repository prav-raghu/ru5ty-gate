use ru5ty_gate_http::RateLimitPolicy;

pub struct RateLimitConfig;

impl RateLimitConfig {
    pub fn global() -> RateLimitPolicy {
        RateLimitPolicy::per_minute(200, "Rate limit exceeded, retry in 1 minute")
    }
}
