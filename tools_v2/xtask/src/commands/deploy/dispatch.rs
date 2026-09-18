use super::cli::DeployCmd;
use anyhow::Result;

pub(crate) fn run(cmd: DeployCmd) -> Result<u8> {
    match cmd {
        DeployCmd::Website { args } => crate::commands::deploy::website::run(&args),
        DeployCmd::Db(db_cmd) => crate::commands::deploy::database_operations::run(db_cmd),
        DeployCmd::Staging { args } => crate::commands::deploy::staging::run(&args),
    }
}
