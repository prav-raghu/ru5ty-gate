use std::borrow::Cow;
use std::error::Error;

use ru5ty_gate_config::EnvReader;

use crate::sentry_config::resolve_sentry_config;

pub fn init_sentry(env: &EnvReader) -> Option<sentry::ClientInitGuard> {
    let config = resolve_sentry_config(env);
    if !config.enabled {
        return None;
    }
    let mut options = sentry::ClientOptions::default();
    options.release = config.release.map(Cow::Owned);
    options.environment = Some(Cow::Owned(config.environment));
    let options = options.traces_sample_rate(config.traces_sample_rate);
    Some(sentry::init((config.dsn, options)))
}

pub fn capture_error(error: &(dyn Error + 'static)) {
    sentry::capture_error(error);
}
