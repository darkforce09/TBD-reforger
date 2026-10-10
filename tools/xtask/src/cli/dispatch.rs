use super::{Cli, TopCmd};
use crate::commands;
use anyhow::Result;
use clap::Parser;

pub(crate) fn run() -> Result<u8> {
    let args = enfusion_mcp::preprocess_cli_args(std::env::args_os().collect());
    let cli = Cli::parse_from(args);
    match cli.cmd {
        TopCmd::Mcp { cmd } => Ok(enfusion_mcp::run(cmd)),
        TopCmd::Debug { cmd } => Ok(remote_debugging::debug::run(cmd)?),
        TopCmd::Repro { cmd } => Ok(remote_debugging::reproduction::run(cmd)?),
        TopCmd::Mod { cmd } => Ok(mod_operations::run(cmd)?),
        TopCmd::Deploy { cmd } => Ok(deployment::run(cmd)?),
        TopCmd::Db { cmd } => Ok(database_operations::local_database::run(cmd)?),
        TopCmd::Setup { cmd } => Ok(workstation_setup::run(cmd)?),
        TopCmd::Staging { cmd } => Ok(staging_procedures::run(cmd)?),
        TopCmd::Fetch { cmd } => commands::fetch::dispatch::run(cmd),
        TopCmd::Ballistics { cmd } => Ok(ballistics_oracle_tooling::run(cmd)?),
        TopCmd::Map { cmd } => commands::map::dispatch::run(cmd),
        TopCmd::Verify { cmd } => commands::verify::dispatch::run(cmd),
        TopCmd::Platform { cmd } => Ok(platform_execution::run(cmd)?),
        TopCmd::Ai { cmd } => commands::agent_context::dispatch::run(cmd),
        TopCmd::Mk { args } => Ok(ci_task_catalog::build_lane::recipes::run(&args)?),
        TopCmd::Ci { target } => {
            let code = ci_task_catalog::task_runner::run(target.as_deref());
            Ok(u8::try_from(code).unwrap_or(1))
        }
        TopCmd::Help => Ok(u8::try_from(ci_task_catalog::task_runner::help()).unwrap_or(1)),
        TopCmd::Gen { cmd } => Ok(schema_tooling::run_gen_command(cmd)?),
        TopCmd::Schema { cmd } => commands::schema::dispatch::run(cmd),
        TopCmd::Refactor { cmd } => commands::refactor::dispatch::run(cmd),
    }
}
