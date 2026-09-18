use super::{Cli, TopCmd};
use crate::core::repository_root::find_repo_root;
use crate::*;
use anyhow::{Result, bail};
use clap::Parser;

pub(crate) fn run() -> Result<u8> {
    let args =
        crate::commands::mcp::workbench_logs::preprocess_cli_args(std::env::args_os().collect());
    let cli = Cli::parse_from(args);
    match cli.cmd {
        TopCmd::Mcp { cmd } => commands::mcp::dispatch::run(cmd),
        TopCmd::Debug { cmd } => commands::debug::dispatch::run(cmd),
        TopCmd::Repro { cmd } => commands::reproduction::dispatch::run(cmd),
        TopCmd::Mod { cmd } => commands::mod_ops::dispatch::run(cmd),
        TopCmd::Deploy { cmd } => commands::deploy::dispatch::run(cmd),
        TopCmd::Db { cmd } => crate::commands::db::operations::run(cmd),
        TopCmd::Setup { cmd } => commands::setup::dispatch::run(cmd),
        TopCmd::Fetch { cmd } => commands::fetch::dispatch::run(cmd),
        TopCmd::Map { cmd } => commands::map::dispatch::run(cmd),
        TopCmd::Verify { cmd } => commands::verify::dispatch::run(cmd),
        TopCmd::SliceCollisions { args } => commands::wave::collisions(&args),
        TopCmd::Wave { cmd } => commands::wave::dispatch::run(cmd),
        TopCmd::Platform { cmd } => commands::platform::dispatch::run(cmd),
        TopCmd::Ai { cmd } => commands::agent_context::dispatch::run(cmd),
        TopCmd::Mk { args } => crate::commands::build::recipes::run(&args),
        TopCmd::Ci { target } => {
            Ok(u8::try_from(crate::commands::ci::task_runner::run(target.as_deref())).unwrap_or(1))
        }
        TopCmd::Help => Ok(u8::try_from(crate::commands::ci::task_runner::help()).unwrap_or(1)),
        TopCmd::Gen { cmd } => commands::generate::dispatch::run(cmd),
        TopCmd::Schema { cmd } => commands::schema::dispatch::run(cmd),
        TopCmd::RegistryGet { field } => {
            let root = find_repo_root()?;
            let reg = load_registry(&root)?;
            match reg.get(&field) {
                Some(serde_json::Value::String(s)) => println!("{s}"),
                Some(serde_json::Value::Number(n)) => println!("{n}"),
                Some(other) => println!("{other}"),
                None => bail!("unknown registry field: {field}"),
            }
            Ok(0)
        }
        TopCmd::Ticket { cmd } => commands::ticket::dispatch::run(cmd),
    }
}
