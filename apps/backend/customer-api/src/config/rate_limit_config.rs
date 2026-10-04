use ru5ty_gate_http::RateLimitPolicy;

pub struct RateLimitConfig;

impl RateLimitConfig {
    pub fn global() -> RateLimitPolicy {
        RateLimitPolicy::per_minute(200, "Rate limit exceeded, retry in 1 minute")
    }

    pub fn auth() -> RateLimitPolicy {
        RateLimitPolicy::per_minute(
            10,
            "Too many authentication attempts. Please try again later.",
        )
    }

    pub fn sensitive_endpoints() -> RateLimitPolicy {
        RateLimitPolicy::per_minute(5, "Rate limit exceeded for this operation.")
    }
}
