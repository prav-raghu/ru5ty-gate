#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_auth::{AuthConfig, TokenService};
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_config::EnvReader;
use ru5ty_gate_types::{Permission, TokenScope};
use uuid::Uuid;

const SECRET_A: &str = "0123456789abcdef0123456789abcdef";
const SECRET_B: &str = "fedcba9876543210fedcba9876543210";

fn config(scope: TokenScope) -> AuthConfig {
    AuthConfig {
        access_secret: SECRET_A.to_owned(),
        refresh_secret: SECRET_B.to_owned(),
        scope,
    }
}

async fn live_service() -> Option<TokenService> {
    let url = std::env::var("TEST_REDIS_URL").ok()?;
    let redis = RedisService::connect(&url, true).await;
    Some(TokenService::new(config(TokenScope::Admin), redis))
}

#[test]
fn config_requires_strong_secrets() {
    let weak = EnvReader::from_pairs([("JWT_SECRET", "short"), ("JWT_REFRESH_SECRET", SECRET_B)]);
    let missing = EnvReader::from_pairs([("JWT_SECRET", SECRET_A)]);
    let ok = EnvReader::from_pairs([("JWT_SECRET", SECRET_A), ("JWT_REFRESH_SECRET", SECRET_B)]);

    assert!(AuthConfig::from_env(&weak, TokenScope::Admin).is_err());
    assert!(AuthConfig::from_env(&missing, TokenScope::Admin).is_err());
    assert!(AuthConfig::from_env(&ok, TokenScope::Admin).is_ok());
}

#[tokio::test]
async fn issued_tokens_verify_with_the_right_secret_and_type() {
    let service = TokenService::new(config(TokenScope::Admin), RedisService::disabled());
    let user_id = Uuid::new_v4().to_string();

    let pair = service
        .generate_token(&user_id, "pat", "Moderator", true)
        .await
        .unwrap();
    let access = service.verify_access_token(&pair.access_token).unwrap();

    assert_eq!(access.id, user_id);
    assert_eq!(access.scope, TokenScope::Admin);
    assert!(access.permissions.contains(&Permission::BatchWrite));
    assert!(service.verify_refresh_token(&pair.refresh_token).is_some());
    assert!(service.verify_access_token(&pair.refresh_token).is_none());
    assert!(service.verify_refresh_token(&pair.access_token).is_none());
    assert!(service.verify_access_token("garbage").is_none());
}

#[tokio::test]
async fn tokens_signed_with_another_secret_are_rejected() {
    let issuer = TokenService::new(config(TokenScope::Admin), RedisService::disabled());
    let other = TokenService::new(
        AuthConfig {
            access_secret: SECRET_B.to_owned(),
            refresh_secret: SECRET_A.to_owned(),
            scope: TokenScope::Admin,
        },
        RedisService::disabled(),
    );
    let pair = issuer
        .generate_token("u", "n", "Support", false)
        .await
        .unwrap();

    assert!(other.verify_access_token(&pair.access_token).is_none());
}

#[tokio::test]
async fn mfa_challenge_tokens_are_not_usable_as_access_tokens() {
    let service = TokenService::new(config(TokenScope::Admin), RedisService::disabled());
    let challenge = service.generate_mfa_challenge_token("user-1").unwrap();

    assert_eq!(
        service.verify_mfa_challenge_token(&challenge).unwrap().id,
        "user-1"
    );
    assert!(service.verify_access_token(&challenge).is_none());
    let pair = service
        .generate_token("user-1", "n", "Support", false)
        .await
        .unwrap();
    assert!(
        service
            .verify_mfa_challenge_token(&pair.access_token)
            .is_none()
    );
}

#[tokio::test]
async fn refresh_requires_redis_and_unknown_roles_get_no_permissions() {
    let service = TokenService::new(config(TokenScope::Customer), RedisService::disabled());
    let pair = service
        .generate_token("u", "n", "Not A Role", false)
        .await
        .unwrap();

    assert!(
        service
            .refresh_token(&pair.refresh_token, false)
            .await
            .is_none()
    );
    assert!(
        service
            .verify_access_token(&pair.access_token)
            .unwrap()
            .permissions
            .is_empty()
    );
    assert!(!service.is_token_blacklisted("anything").await);
}

#[tokio::test]
async fn refresh_rotates_tokens_and_rejects_replay_when_redis_is_configured() {
    let Some(service) = live_service().await else {
        return;
    };
    let user_id = Uuid::new_v4().to_string();
    let first = service
        .generate_token(&user_id, "pat", "Moderator", false)
        .await
        .unwrap();

    let second = service
        .refresh_token(&first.refresh_token, false)
        .await
        .unwrap();

    assert_ne!(first.refresh_token_id, second.refresh_token_id);
    assert!(
        service
            .refresh_token(&first.refresh_token, false)
            .await
            .is_none()
    );
    assert!(
        service
            .refresh_token(&second.refresh_token, false)
            .await
            .is_some()
    );
    service.logout(&user_id, None, None).await;
}

#[tokio::test]
async fn logout_revokes_every_session_for_the_user_when_redis_is_configured() {
    let Some(service) = live_service().await else {
        return;
    };
    let user_id = Uuid::new_v4().to_string();
    let pair = service
        .generate_token(&user_id, "pat", "Support", true)
        .await
        .unwrap();
    let other_device = service
        .generate_token(&user_id, "pat", "Support", true)
        .await
        .unwrap();
    let access = service.verify_access_token(&pair.access_token).unwrap();

    service
        .logout(
            &user_id,
            Some(&pair.access_token),
            Some(&pair.refresh_token),
        )
        .await;

    assert!(service.is_token_blacklisted(&access.jti).await);
    assert!(
        service
            .is_session_invalidated(&user_id, access.iat - 1)
            .await
    );
    assert!(
        service
            .refresh_token(&other_device.refresh_token, true)
            .await
            .is_none()
    );
    assert!(
        service
            .refresh_token(&pair.refresh_token, true)
            .await
            .is_none()
    );
}
