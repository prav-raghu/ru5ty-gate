use std::process::ExitCode;

use api_gateway::application::Application;
use api_gateway::config::ServiceConfig;
use ru5ty_gate_config::EnvReader;
use ru5ty_gate_http::run_healthcheck;
use ru5ty_gate_logging::init_logging;
use ru5ty_gate_observability::init_sentry;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    let env = EnvReader::from_process();

    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let port = env.parse_or("PORT", 4000_u16).unwrap_or(4000);
        return if run_healthcheck(port, "/health/live") {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    let _sentry = init_sentry(&env);
    if init_logging("api-gateway").is_err() {
        return ExitCode::FAILURE;
    }
    let config = match ServiceConfig::from_env(&env) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "invalid configuration");
            return ExitCode::FAILURE;
        }
    };
    let application = match Application::initialize(config) {
        Ok(application) => application,
        Err(error) => {
            tracing::error!(%error, "failed to initialise api-gateway");
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
