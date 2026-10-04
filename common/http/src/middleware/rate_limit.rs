use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::app_error::AppError;
use crate::extractors::ClientIp;
use crate::middleware::auth::AuthUser;

const CLEANUP_THRESHOLD: usize = 10_000;

#[derive(Debug, Clone)]
pub struct RateLimitPolicy {
    pub max: u32,
    pub window: Duration,
    pub message: String,
}

impl RateLimitPolicy {
    pub fn per_minute(max: u32, message: &str) -> Self {
        Self {
            max,
            window: Duration::from_secs(60),
            message: message.to_owned(),
        }
    }
}

pub struct RateLimiter {
    policy: RateLimitPolicy,
    hits: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(policy: RateLimitPolicy) -> Arc<Self> {
        Arc::new(Self {
            policy,
            hits: Mutex::new(HashMap::new()),
        })
    }

    pub fn check_at(&self, key: &str, now: Instant) -> Result<(), u64> {
        let mut hits = self.hits.lock().unwrap_or_else(PoisonError::into_inner);
        if hits.len() > CLEANUP_THRESHOLD {
            hits.retain(|_, (started, _)| now.duration_since(*started) < self.policy.window);
        }
        let entry = hits.entry(key.to_owned()).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.policy.window {
            *entry = (now, 0);
        }
        if entry.1 >= self.policy.max {
            let remaining = self
                .policy
                .window
                .saturating_sub(now.duration_since(entry.0));
            return Err(remaining.as_secs().max(1));
        }
        entry.1 += 1;
        Ok(())
    }

    pub fn check(&self, key: &str) -> Result<(), u64> {
        self.check_at(key, Instant::now())
    }

    pub fn message(&self) -> &str {
        &self.policy.message
    }
}

pub async fn rate_limit(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let key = match request.extensions().get::<AuthUser>() {
        Some(user) => format!("user:{}", user.id),
        None => {
            let (parts, body) = request.into_parts();
            let address = ClientIp::resolve(&parts);
            let request = Request::from_parts(parts, body);
            return limited(&limiter, &format!("ip:{address}"), request, next).await;
        }
    };
    limited(&limiter, &key, request, next).await
}

async fn limited(limiter: &RateLimiter, key: &str, request: Request, next: Next) -> Response {
    match limiter.check(key) {
        Ok(()) => next.run(request).await,
        Err(retry_after_seconds) => AppError::TooManyRequests {
            message: limiter.message().to_owned(),
            retry_after_seconds,
        }
        .into_response(),
    }
}
