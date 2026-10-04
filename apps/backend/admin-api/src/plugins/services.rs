use std::sync::Arc;

use ru5ty_gate_auth::TokenService;
use ru5ty_gate_cache::RedisService;
use ru5ty_gate_database::PgPool;
use ru5ty_gate_email::EmailSender;
use ru5ty_gate_utilities::CryptoUtil;

use crate::config::ServiceConfig;
use crate::services::{
    AuthService, BatchOperationService, BootstrapService, ReportingService, UserService,
};
use crate::types::Services;

pub fn build_services(
    config: &ServiceConfig,
    pool: &PgPool,
    redis: &RedisService,
    email: Arc<dyn EmailSender>,
) -> Result<Services, ru5ty_gate_utilities::CryptoError> {
    let crypto = Arc::new(CryptoUtil::from_hex_key(&config.two_factor_encryption_key)?);
    let token = TokenService::new(config.auth.clone(), redis.clone());
    let user = UserService::new(
        pool.clone(),
        email.clone(),
        crypto,
        config.admin_web_url.clone(),
    );
    let auth = AuthService::new(
        pool.clone(),
        token.clone(),
        email,
        redis.clone(),
        user.clone(),
        config.admin_web_url.clone(),
        config.password_reset_expiration_minutes,
    );
    Ok(Services {
        token,
        user,
        auth,
        batch: BatchOperationService::new(pool.clone()),
        reporting: ReportingService::new(pool.clone()),
        bootstrap: BootstrapService::new(pool.clone(), config.admin_bootstrap_enabled),
    })
}
