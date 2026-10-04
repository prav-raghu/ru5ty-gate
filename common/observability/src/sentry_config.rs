use ru5ty_gate_config::EnvReader;

const DEFAULT_APP_ENV: &str = "development";
const DEFAULT_TRACES_SAMPLE_RATE: f32 = 0.1;

#[derive(Debug, Clone, PartialEq)]
pub struct SentryConfig {
    pub dsn: String,
    pub environment: String,
    pub release: Option<String>,
    pub traces_sample_rate: f32,
    pub enabled: bool,
}

pub fn resolve_sentry_config(env: &EnvReader) -> SentryConfig {
    let dsn = env.optional("SENTRY_DSN").unwrap_or_default();
    let traces_sample_rate = env
        .optional("SENTRY_TRACES_SAMPLE_RATE")
        .and_then(|raw| raw.parse::<f32>().ok())
        .filter(|rate| (0.0..=1.0).contains(rate))
        .unwrap_or(DEFAULT_TRACES_SAMPLE_RATE);
    SentryConfig {
        enabled: !dsn.is_empty(),
        dsn,
        environment: env
            .optional("APP_ENV")
            .unwrap_or_else(|| DEFAULT_APP_ENV.to_owned()),
        release: env.optional("SENTRY_RELEASE"),
        traces_sample_rate,
    }
}
