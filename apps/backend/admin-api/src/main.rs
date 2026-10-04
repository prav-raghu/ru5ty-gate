use std::process::ExitCode;

use admin_api::application::Application;
use admin_api::config::ServiceConfig;
use ru5ty_gate_config::EnvReader;
use ru5ty_gate_database::{DatabaseConfig, connect, run_migrations};
use ru5ty_gate_http::run_healthcheck;
use ru5ty_gate_logging::init_logging;
use ru5ty_gate_observability::init_sentry;

async fn migrate(env: &EnvReader) -> ExitCode {
    let config = match DatabaseConfig::from_env(env) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("invalid database configuration: {error}");
            return ExitCode::FAILURE;
        }
    };
    let result = async {
        let pool = connect(&config).await?;
        run_migrations(&pool).await
    }
    .await;
    match result {
        Ok(()) => {
            println!("Migrations applied");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("migration failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    let env = EnvReader::from_process();

    match std::env::args().nth(1).as_deref() {
        Some("healthcheck") => {
            let port = env.parse_or("PORT", 4001_u16).unwrap_or(4001);
            return if run_healthcheck(port, "/api/v1/ping") {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
        Some("migrate") => return migrate(&env).await,
        _ => {}
    }

    let _sentry = init_sentry(&env);
    if init_logging("admin-api").is_err() {
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
            tracing::error!(%error, "failed to initialise admin-api");
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
