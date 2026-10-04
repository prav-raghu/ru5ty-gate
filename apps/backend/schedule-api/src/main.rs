use std::process::ExitCode;

use ru5ty_gate_config::EnvReader;
use ru5ty_gate_http::run_healthcheck;
use ru5ty_gate_logging::init_logging;
use ru5ty_gate_observability::init_sentry;
use schedule_api::application::Application;
use schedule_api::config::ServiceConfig;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    let env = EnvReader::from_process();

    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let port = env.parse_or("PORT", 4003_u16).unwrap_or(4003);
        return if run_healthcheck(port, "/api/v1/ping") {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let _sentry = init_sentry(&env);
    if init_logging("schedule-api").is_err() {
        return ExitCode::FAILURE;
    }
    let config = match ServiceConfig::from_env(&env) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "invalid configuration");
            return ExitCode::FAILURE;
        }
    };
    let application = match Application::initialize(config).await {
        Ok(application) => application,
        Err(error) => {
            tracing::error!(%error, "failed to initialise schedule-api");
            return ExitCode::FAILURE;
        }
    };
    match application.start().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "server stopped unexpectedly");
            ExitCode::FAILURE
        }
    }
}
