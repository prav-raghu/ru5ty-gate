use ru5ty_gate_config::EnvReader;
use thiserror::Error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("logging has already been initialised")]
    AlreadyInitialised,
}

fn directive_for(level: &str) -> &'static str {
    match level {
        "trace" => "trace",
        "debug" => "debug",
        "warn" => "warn",
        "error" | "fatal" => "error",
        "silent" => "off",
        _ => "info",
    }
}

pub fn init_logging(service_name: &str) -> Result<(), LoggingError> {
    let env = EnvReader::from_process();
    let level = env
        .optional("LOG_LEVEL")
        .unwrap_or_else(|| "info".to_owned());
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directive_for(&level)));
    let production = env
        .optional("APP_ENV")
        .is_some_and(|value| value == "production");

    let registry = tracing_subscriber::registry().with(filter);
    let result = if production {
        registry
            .with(tracing_subscriber::fmt::layer().json().flatten_event(true))
            .try_init()
    } else {
        registry.with(tracing_subscriber::fmt::layer()).try_init()
    };

    result.map_err(|_| LoggingError::AlreadyInitialised)?;
    tracing::info!(service = service_name, "logging initialised");
    Ok(())
}
