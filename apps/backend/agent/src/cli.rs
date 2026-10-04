use clap::Parser;
use ru5ty_gate_agent_config::{ConfigError, Settings};

use crate::AGENT_VERSION;

#[derive(Parser, Debug)]
#[command(name = "ru5ty-gate-agent", version = AGENT_VERSION)]
pub struct Cli {
    #[arg(short, long)]
    pub config: Option<String>,
}

impl Cli {
    pub fn load_settings(&self) -> Result<Settings, ConfigError> {
        match &self.config {
            Some(path) => Settings::load_from(path),
            None => Settings::load(),
        }
    }
}
