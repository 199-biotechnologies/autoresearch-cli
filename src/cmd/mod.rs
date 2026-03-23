mod agent_info;
mod best;
mod diff;
mod doctor;
mod export;
mod init;
mod install;
mod log;
mod record;
mod report;
mod status;

use crate::cli::{Cli, Commands};
use crate::errors::CliError;

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Install { target } => install::run(&target, cli.json),
        Commands::Init {
            target_file,
            eval_command,
            metric_name,
            metric_direction,
            time_budget,
            branch,
        } => init::run(
            target_file,
            eval_command,
            &metric_name,
            &metric_direction,
            &time_budget,
            &branch,
            cli.json,
        ),
        Commands::Record {
            metric,
            status,
            summary,
        } => record::run(metric, &status, &summary, cli.json),
        Commands::Log { limit } => log::run(limit, cli.json),
        Commands::Best => best::run(cli.json),
        Commands::Diff { run_a, run_b } => diff::run(run_a, run_b, cli.json),
        Commands::Status => status::run(cli.json),
        Commands::Export { format, output } => export::run(&format, output.as_deref(), cli.json),
        Commands::Doctor => doctor::run(cli.json),
        Commands::Report { output } => report::run(output.as_deref(), cli.json),
        Commands::AgentInfo => agent_info::run(cli.json),
    }
}
