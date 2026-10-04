use clap::Subcommand;

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum Command {
    Healthcheck,
}
