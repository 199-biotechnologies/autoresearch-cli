mod agent_info;
mod best;
mod diff;
mod export;
mod init;
mod install;
mod log;
mod status;

use crate::cli::{Cli, Commands};
use crate::errors::CliError;

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Install { target } => install::run(&target, cli.json),
        Commands::Init => init::run(cli.json),
        Commands::Log { limit } => log::run(limit, cli.json),
        Commands::Best => best::run(cli.json),
        Commands::Diff { run_a, run_b } => diff::run(run_a, run_b, cli.json),
        Commands::Status => status::run(cli.json),
        Commands::Export { format, output } => export::run(&format, output.as_deref(), cli.json),
        Commands::AgentInfo => agent_info::run(cli.json),
    }
}
