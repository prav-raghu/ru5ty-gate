use std::process::ExitCode;

use clap::Parser;
use ru5ty_gate_agent::{Application, Cli};
use ru5ty_gate_logging::init_logging;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if init_logging("ru5ty-gate-agent").is_err() {
        return ExitCode::FAILURE;
    }

    let settings = match cli.load_settings() {
        Ok(settings) => settings,
        Err(error) => {
            tracing::error!(%error, "invalid configuration");
            return ExitCode::FAILURE;
        }
    };

    let application = match Application::initialize(settings) {
        Ok(application) => application,
        Err(error) => {
            tracing::error!(%error, "failed to initialise ru5ty-gate-agent");
            return ExitCode::FAILURE;
        }
    };

    match application.start().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "agent stopped unexpectedly");
            ExitCode::FAILURE
        }
    }
}
