//! The `cargo xtask setup` dispatch: one setup command to the module that runs it.
//!
//! **Role:** [`run`] maps a parsed [`SetupCmd`] to its command and returns that command's exit
//! code.
//! **Position:** the xtask binary's dispatch calls it with the parsed `setup` subcommand.
//! **Signals & state:** none; each command owns its own effects.
//! **Invariants:** every [`SetupCmd`] variant reaches exactly one command, with its arguments as
//! parsed.

use crate::SetupCmd;
use crate::error::Result;

/// Runs one `cargo xtask setup` command and returns its exit code.
pub fn run(cmd: SetupCmd) -> Result<u8> {
    match cmd {
        SetupCmd::ServerProfile { profile } => crate::server_profile::run(profile.as_deref()),
        SetupCmd::Workbench => crate::workbench_linux::run(),
        SetupCmd::McpGameRoot { game, fake } => {
            crate::mcp_game_root::run(game.as_deref(), fake.as_deref())
        }
        SetupCmd::ClientAddons => crate::client_addons::run(),
    }
}
