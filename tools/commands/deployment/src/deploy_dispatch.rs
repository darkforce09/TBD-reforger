//! Runs one `cargo xtask deploy` command.
//!
//! **Role:** [`run`] hands each [`DeployCmd`] to its driver.
//! **Position:** called by the xtask binary's `deploy` group.
//! **Signals & state:** none.
//! **Invariants:** the exit code is the driver's own.

use crate::deploy_command::DeployCmd;
use crate::error::Result;

/// Runs one `cargo xtask deploy` command and returns its exit code.
pub fn run(cmd: DeployCmd) -> Result<u8> {
    match cmd {
        DeployCmd::Website { args } => crate::website::run(&args),
        DeployCmd::Db(db_cmd) => Ok(database_operations::container_database::run(db_cmd)?),
        DeployCmd::Staging { args } => crate::staging::run(&args),
    }
}
