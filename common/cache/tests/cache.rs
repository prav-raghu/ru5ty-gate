#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_cache::{CacheError, RedisService, resolve_redis_url};

#[test]
fn insecure_tls_is_opt_in_for_rediss_urls() {
    assert_eq!(
        resolve_redis_url("rediss://cache:6380", false),
        "rediss://cache:6380#insecure"
    );
    assert_eq!(
        resolve_redis_url("rediss://cache:6380", true),
        "rediss://cache:6380"
    );
    assert_eq!(
        resolve_redis_url("redis://cache:6379", false),
        "redis://cache:6379"
    );
}

#[tokio::test]
async fn disabled_service_reports_unavailable() {
    let service = RedisService::disabled();

    assert!(!service.is_available());
    assert!(matches!(
        service.get("key").await,
        Err(CacheError::Unavailable)
    ));
    assert!(matches!(service.ping().await, Err(CacheError::Unavailable)));
}

#[tokio::test]
async fn unreachable_server_falls_back_to_disabled() {
    let service = RedisService::connect("redis://127.0.0.1:1", true).await;

    assert!(!service.is_available());
}

#[tokio::test]
async fn round_trips_against_a_live_server_when_configured() {
    let Ok(url) = std::env::var("TEST_REDIS_URL") else {
        return;
    };
    let service = RedisService::connect(&url, true).await;
    let key = format!("cache-test:{}", std::process::id());

    service.set_ex(&key, 30, "value").await.unwrap();

    assert_eq!(service.get(&key).await.unwrap().as_deref(), Some("value"));
    assert!(service.exists(&key).await.unwrap());
    assert_eq!(service.incr(&format!("{key}:n")).await.unwrap(), 1);
    assert!(
        service
            .keys_matching(&format!("{key}*"))
            .await
            .unwrap()
            .contains(&key)
    );
    service.del(&key).await.unwrap();
    service.del(&format!("{key}:n")).await.unwrap();
    assert!(!service.exists(&key).await.unwrap());
}
