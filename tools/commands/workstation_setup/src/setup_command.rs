//! The `cargo xtask setup` subcommands as the command line spells them.
//!
//! **Role:** [`SetupCmd`], the clap subcommand of the `setup` group, with each command's optional
//! paths.
//! **Position:** the xtask binary's command line embeds it under `setup`; [`crate::run`] takes
//! the parsed value.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant names one command, and an omitted path keeps the command's own
//! default.

use clap::Subcommand;
use std::path::PathBuf;

/// One `cargo xtask setup` command.
#[derive(Subcommand, Debug)]
pub enum SetupCmd {
    /// Prepare Arma Reforger dedicated-server profile files.
    #[command(name = "server-profile")]
    ServerProfile {
        /// Profile directory (default: $TBD_PROFILE or mod/.local-test-profile)
        profile: Option<PathBuf>,
    },
    /// Symlink Steam Arma Reforger .gproj for Proton Workbench.
    #[command(name = "workbench")]
    Workbench,
    /// Flattened pak symlink farm for enfusion-mcp.
    #[command(name = "mcp-game-root")]
    McpGameRoot {
        /// Game install with addons/ (default: $HOME/.local/share/Steam/steamapps/common/Arma Reforger)
        game: Option<PathBuf>,
        /// Output symlink farm (default: $HOME/.cache/enfusion-mcp-root)
        fake: Option<PathBuf>,
    },
    /// Local client addon staging symlink + Steam launch options.
    #[command(name = "client-addons")]
    ClientAddons,
}
