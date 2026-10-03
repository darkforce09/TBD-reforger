//! The `cargo xtask deploy` commands as the command line spells them.
//!
//! **Role:** [`DeployCmd`], the clap subcommand of the `deploy` group.
//! **Position:** the xtask binary's command line embeds it under `deploy`; [`crate::run`] takes
//! the parsed value.
//! **Signals & state:** none; plain data.
//! **Invariants:** the website and staging drivers parse their own arguments, so their argv
//! reaches them untouched.

use clap::Subcommand;

/// One `cargo xtask deploy` command.
#[derive(Subcommand, Debug)]
pub enum DeployCmd {
    /// Rsync + remote build/restart for the TBD website.
    #[command(name = "website", disable_help_flag = true)]
    Website {
        /// The website driver's own arguments, passed through.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Shared database backup/restore plumbing for the deploy drivers.
    #[command(subcommand)]
    Db(database_operations::container_database::DeployDbCmd),
    /// Staging deploy driver for the dedicated game server
    #[command(name = "staging", disable_help_flag = true)]
    Staging {
        /// The staging driver's own arguments, passed through.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
